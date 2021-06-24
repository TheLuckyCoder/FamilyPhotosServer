package net.theluckycoder.familyphotos.controller

import net.coobird.thumbnailator.Thumbnails
import net.theluckycoder.familyphotos.StartData
import net.theluckycoder.familyphotos.exceptions.FileStorageException
import net.theluckycoder.familyphotos.exceptions.PhotoNotFoundException
import net.theluckycoder.familyphotos.extensions.LoggerExtensions
import net.theluckycoder.familyphotos.extensions.getMimeTypeAll
import net.theluckycoder.familyphotos.model.Photo
import net.theluckycoder.familyphotos.repository.PhotoRepository
import net.theluckycoder.familyphotos.repository.UserRepository
import net.theluckycoder.familyphotos.repository.findByIdOrThrow
import net.theluckycoder.familyphotos.service.FileStorageService
import org.apache.tomcat.util.http.fileupload.ByteArrayOutputStream
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.core.io.ByteArrayResource
import org.springframework.core.io.Resource
import org.springframework.data.repository.findByIdOrNull
import org.springframework.http.HttpHeaders
import org.springframework.http.InvalidMediaTypeException
import org.springframework.http.MediaType
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestParam
import org.springframework.web.bind.annotation.RestController
import org.springframework.web.multipart.MultipartFile
import javax.servlet.http.HttpServletRequest

@RestController
class PhotosController @Autowired constructor(
    private val userRepository: UserRepository,
    private val photoRepository: PhotoRepository,
    private val fileStorageService: FileStorageService,
) {

    private val logger = LoggerExtensions.getLogger<PhotosController>()

    private val publicUser by lazy { userRepository.findByUserName(StartData.PUBLIC_USERNAME).get() }

    @GetMapping("/photos/{userId}")
    fun getPhotosList(@PathVariable userId: String): Iterable<Photo> =
        photoRepository.findByUser(userId.toLong())

    @GetMapping("/photos/{userId}/download/{photoId}")
    fun downloadPhoto(
        @PathVariable userId: String,
        @PathVariable photoId: String,
        @RequestParam(defaultValue = "false") thumbnail: String,
        request: HttpServletRequest
    ): ResponseEntity<Resource?> {
        val userIdLong = userId.toLong()
        val photoIdLong = photoId.toLong()
        val thumbnailRequested = thumbnail.toBoolean()

        val user = userRepository.findByIdOrThrow(userIdLong)
        val photo = photoRepository.findByIdOrNull(photoIdLong)
        if (photo == null || photo.ownerUserId != userIdLong)
            throw PhotoNotFoundException("There is no Photo with id $photoId")

        val fileName = photo.getStorePath(user)

        // Load file as Resource
        var resource = fileStorageService.loadFileAsResource(fileName)

        // Try to determine file's content type
        var contentType = try {
            request.servletContext.getMimeTypeAll(resource.file)
        } catch (e: InvalidMediaTypeException) {
            logger.warn("Could not automatically determine file type.", e)
            // Fallback to the default content type if type could not be determined
            "image/*"
        }

        if (thumbnailRequested && contentType.startsWith("image/") && contentType != "image/gif") {
            logger.info("Photo ${photo.id} requested by user $userIdLong (Thumbnail)")
            try {
                val outputStream = ByteArrayOutputStream()
                Thumbnails.of(fileStorageService.getFile(fileName))
                    .size(300, 300)
                    .outputFormat("jpeg")
                    .toOutputStream(outputStream)

                resource = ByteArrayResource(outputStream.toByteArray())
                contentType = "image/jpeg"
            } catch (e: Exception) {
                e.printStackTrace()
            }
        } else {
            logger.info("Photo ${photo.id} requested by user $userIdLong")
        }

        return ResponseEntity.ok()
            .contentType(MediaType.parseMediaType(contentType))
            .header(
                HttpHeaders.CONTENT_DISPOSITION,
                "attachment; filename=\"${resource.filename}\""
            )
            .body<Resource?>(resource)
    }

    @PostMapping("/photos/{userId}/upload")
    fun uploadPhoto(
        @PathVariable userId: String,
        @RequestParam("file") file: MultipartFile,
        @RequestParam("timeCreated") timeCreated: String,
    ): Photo {
        val userIdLong = userId.toLong()
        val timestampCreated = timeCreated.toLong()
        require(timestampCreated > 0) { "Invalid photo creation timestamp" }
//        require(file.contentType!!.startsWith("image/")) { "Uploaded file has to be an image or a video" }

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
        logger.info("Saving Photo $photo to $filePath")

        fileStorageService.storeFile(file, filePath)
        fileStorageService.setCreationTime(filePath, timestampCreated)

        return photo
    }

    @PostMapping("/photos/{userId}/delete/{photoId}")
    fun deletePhoto(
        @PathVariable userId: String,
        @PathVariable photoId: String,
    ): ResponseEntity<Void> {
        val userIdLong = userId.toLong()
        val photoIdLong = photoId.toLong()

        val user = userRepository.findByIdOrThrow(userIdLong)
        val photo = photoRepository.findByIdOrNull(photoIdLong)
        if (photo == null || photo.ownerUserId != userIdLong)
            throw PhotoNotFoundException("There is no Photo with id $photoId")

        val path = photo.getStorePath(user)

        if (fileStorageService.existsFile(path)) {
            if (!fileStorageService.deleteFile(path)) {
                logger.error("Failed to delete file {$path}")
                throw FileStorageException("Delete operation failed")
            }
        } else {
            throw PhotoNotFoundException("There is no such Photo existent on disk")
        }

        return ResponseEntity.ok().build()
    }

    // region Public

    @GetMapping("/public_photos/")
    fun getPublicPhotosList(): Iterable<Photo> =
        getPhotosList(publicUser.id.toString())

    @PostMapping("/public_photos/make_public/{userId}/{photoId}")
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
            logger.error("Failed to move file from {$fromFile} to {$toFile}")
            throw FileStorageException("Move operation failed")
        }

        photoRepository.save(publicPhoto)

        return ResponseEntity.ok().build()
    }

    @GetMapping("/public_photos/download/{photoId}")
    fun downloadPublicPhoto(
        @PathVariable photoId: String,
        request: HttpServletRequest
    ) = downloadPhoto(publicUser.id.toString(), photoId, false.toString(), request)

    @PostMapping("/public_photos/delete/{photoId}")
    fun deletePublicPhoto(
        @PathVariable photoId: String,
    ) = deletePhoto(publicUser.id.toString(), photoId)

    // endregion Public
}
