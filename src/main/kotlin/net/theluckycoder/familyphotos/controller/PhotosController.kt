package net.theluckycoder.familyphotos.controller

import net.theluckycoder.familyphotos.StartData
import net.theluckycoder.familyphotos.exceptions.FileStorageException
import net.theluckycoder.familyphotos.exceptions.PhotoNotFoundException
import net.theluckycoder.familyphotos.extensions.LoggerExtensions
import net.theluckycoder.familyphotos.extensions.getMimeTypeAll
import net.theluckycoder.familyphotos.model.Photo
import net.theluckycoder.familyphotos.model.User
import net.theluckycoder.familyphotos.repository.PhotoRepository
import net.theluckycoder.familyphotos.repository.UserRepository
import net.theluckycoder.familyphotos.repository.findByIdOrThrow
import net.theluckycoder.familyphotos.service.FileStorageService
import net.theluckycoder.familyphotos.utils.Md5ShallowEtagHeaderFilter
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.data.repository.findByIdOrNull
import org.springframework.http.CacheControl
import org.springframework.http.HttpHeaders
import org.springframework.http.HttpHeaders.IF_NONE_MATCH
import org.springframework.http.HttpStatus
import org.springframework.http.InvalidMediaTypeException
import org.springframework.http.MediaType
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestHeader
import org.springframework.web.bind.annotation.RequestParam
import org.springframework.web.bind.annotation.RestController
import org.springframework.web.multipart.MultipartFile
import org.springframework.web.servlet.mvc.method.annotation.StreamingResponseBody
import java.io.OutputStream
import java.nio.file.Files
import java.util.*
import java.util.concurrent.ConcurrentHashMap
import javax.servlet.http.HttpServletRequest

@RestController
class PhotosController @Autowired constructor(
    private val userRepository: UserRepository,
    private val photoRepository: PhotoRepository,
    private val fileStorageService: FileStorageService,
) {

    private val log = LoggerExtensions.getLogger<PhotosController>()
    private val etagHeaderFilter = Md5ShallowEtagHeaderFilter()
    private val publicUser by lazy { userRepository.findByUserName(StartData.PUBLIC_USERNAME).get() }

    private val photoEtagMap = ConcurrentHashMap<Long, String>(1024)
    private val cacheControl = CacheControl.empty().cachePrivate().mustRevalidate()

    // We assume the photos won't be changed since they can't be right now
    private fun getFileEtag(photo: Photo, user: User): String {
        return photoEtagMap.getOrPut(photo.id) {
            fileStorageService.resolveFileName(photo.getStorePath(user)).inputStream().use {
                etagHeaderFilter.generateETagHeaderValue(it)
            }
        }
    }

    @GetMapping("/photos/{userId}")
    fun getPhotosList(@PathVariable userId: String): Iterable<Photo> =
        photoRepository.findByUser(userId.toLong())

    @GetMapping("/photos/{userId}/download/{photoId}")
    fun downloadPhoto(
        @PathVariable userId: String,
        @PathVariable photoId: String,
        request: HttpServletRequest,
        @RequestHeader(IF_NONE_MATCH) requestEtagOpt: Optional<String>,
    ): ResponseEntity<StreamingResponseBody> {

        val userIdLong = userId.toLong()
        val photoIdLong = photoId.toLong()

        val user = userRepository.findByIdOrThrow(userIdLong)
        val photo = photoRepository.findByIdOrNull(photoIdLong)
        if (photo == null || photo.ownerUserId != userIdLong)
            throw PhotoNotFoundException("There is no Photo with id $photoId")

        val serverEtag = getFileEtag(photo, user)
        if (!requestEtagOpt.isEmpty && requestEtagOpt.get() == serverEtag) {
            log.info("Photo ${photo.id} requested by user $userIdLong, cached on client side")

            return ResponseEntity
                .status(HttpStatus.NOT_MODIFIED)
                .eTag(serverEtag)
                .cacheControl(cacheControl)
                .body(null)
        }

        val fileName = photo.getStorePath(user)
        val file = fileStorageService.resolveFileName(fileName)

        // Try to determine file's content type
        val contentType = try {
            request.servletContext.getMimeTypeAll(file)
        } catch (e: InvalidMediaTypeException) {
            log.warn("Could not automatically determine file type.", e)
            // Fallback to the default content type if type could not be determined
            "image/*"
        }

        val responseBody = StreamingResponseBody { outputStream: OutputStream ->
            try {
                Files.copy(file.toPath(), outputStream)
            } catch (_: Exception) {
            }
        }
        log.info("Photo ${photo.id} requested by user $userIdLong")

        return ResponseEntity.ok()
            .contentType(MediaType.parseMediaType(contentType))
            .header(
                HttpHeaders.CONTENT_DISPOSITION,
                "attachment; filename=\"${file.name}\""
            )
            .cacheControl(cacheControl)
            .eTag(serverEtag)
            .body(responseBody)
    }

    @PostMapping("/photos/{userId}")
    fun uploadPhoto(
        @PathVariable userId: String,
        @RequestParam("file") file: MultipartFile,
        @RequestParam("timeCreated") timeCreated: String,
    ): Photo {
        val userIdLong = userId.toLong()
        val timestampCreated = timeCreated.toLong()
        require(timestampCreated > 0) { "Invalid photo creation timestamp" }
        require(file.contentType!!.startsWith("image/")) { "Uploaded file has to be an image or a video" }

        val user = userRepository.findByIdOrThrow(userIdLong)
        val simpleFileName = file.originalFilename!!.substringAfterLast('/')
        val name = simpleFileName.substringBeforeLast('.') +
                "-${System.currentTimeMillis()}." +
                simpleFileName.substringAfterLast('.')

        val photo = photoRepository.save(
            Photo(
                ownerUserId = userIdLong,
                name = name,
                timeCreated = timestampCreated,
                fileSize = file.size,
            )
        )

        val filePath = photo.getStorePath(user)
        log.info("Saving Photo $photo to $filePath")

        fileStorageService.storeFile(file, filePath)
        fileStorageService.setCreationTime(filePath, timestampCreated)

        return photo
    }

    @GetMapping("/photos/{userId}/delete/{photoId}")
    fun deletePhoto(
        @PathVariable userId: String,
        @PathVariable photoId: String,
    ): Map<String, Boolean> {
        val userIdLong = userId.toLong()
        val photoIdLong = photoId.toLong()

        val user = userRepository.findByIdOrThrow(userIdLong)
        val photo = photoRepository.findByIdOrNull(photoIdLong)
        if (photo == null || photo.ownerUserId != userIdLong)
            throw PhotoNotFoundException("There is no Photo with id $photoId")

        val path = photo.getStorePath(user)

        log.info("Photo ${photo.id} by user $userIdLong, to be deleted")

        if (!fileStorageService.existsFile(path))
            throw PhotoNotFoundException("There is no such Photo existent on disk")

        if (!fileStorageService.deleteFile(path)) {
            log.error("Failed to delete file {$path}")
            throw FileStorageException("Delete operation failed")
        }

        photoRepository.delete(photo)
        log.info("Photo ${photo.id} by user $userIdLong, deleted successfully")

        return mapOf("deleted" to true)
    }

    // region Public

    @GetMapping("/public_photos/")
    fun getPublicPhotosList(): Iterable<Photo> =
        getPhotosList(publicUser.id.toString())

    @PostMapping("/photos/{userId}/make_public/{photoId}")
    fun makePhotoPublic(
        @PathVariable userId: String,
        @PathVariable photoId: String,
    ): ResponseEntity<Void> {
        val userIdLong = userId.toLong()
        val photoIdLong = photoId.toLong()

        val user = userRepository.findByIdOrThrow(userIdLong)
        val photo = photoRepository.findByIdOrNull(photoIdLong)
        if (photo == null || photo.ownerUserId != userIdLong)
            throw PhotoNotFoundException("There is no Photo with id $photoId")

        val publicPhoto = photo.copy(
            ownerUserId = publicUser.id,
        )

        val fromFile = photo.getStorePath(user)
        val toFile = publicPhoto.getStorePath(publicUser)
        if (!fileStorageService.moveFile(fromFile, toFile)) {
            log.error("Failed to move file from {$fromFile} to {$toFile}")
            throw FileStorageException("Move operation failed")
        }

        photoRepository.save(publicPhoto)

        return ResponseEntity.ok().build()
    }

    @GetMapping("/public_photos/download/{photoId}")
    fun downloadPublicPhoto(
        @PathVariable photoId: String,
        request: HttpServletRequest,
        @RequestHeader(IF_NONE_MATCH) requestEtagOpt: Optional<String>,
    ) = downloadPhoto(publicUser.id.toString(), photoId, request, requestEtagOpt)

    @GetMapping("/public_photos/delete/{photoId}")
    fun deletePublicPhoto(
        @PathVariable photoId: String,
    ) = deletePhoto(publicUser.id.toString(), photoId)

    // endregion Public
}
