package net.theluckycoder.familyphotos.service

import net.theluckycoder.familyphotos.configs.FileStorageProperties
import net.theluckycoder.familyphotos.exceptions.FileStorageException
import net.theluckycoder.familyphotos.exceptions.PhotoNotFoundException
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.core.io.Resource
import org.springframework.core.io.UrlResource
import org.springframework.stereotype.Service
import org.springframework.web.multipart.MultipartFile
import java.io.IOException
import java.net.MalformedURLException
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import java.nio.file.attribute.BasicFileAttributeView
import java.nio.file.attribute.FileTime

@Service
class FileStorageService @Autowired constructor(fileStorageProperties: FileStorageProperties) {

    private val fileStorageLocation = Path.of(fileStorageProperties.storageDir)

    init {
        try {
            Files.createDirectories(fileStorageLocation)
        } catch (e: Exception) {
            throw FileStorageException(
                "Could not create the directory where the uploaded files will be stored. $fileStorageLocation",
                e
            )
        }
    }

    fun resolveFileName(fileName: String) = fileStorageLocation.resolve(fileName).toFile()

    fun storeFile(file: MultipartFile, fileName: String) {
        try {
            // Check if the file's name contains invalid characters
            if (fileName.contains("..")) {
                throw FileStorageException("Sorry! Filename contains invalid path sequence $fileName")
            }

            // Copy file to the target location (Replacing existing file with the same name)
            val targetLocation = fileStorageLocation.resolve(fileName)

            val parent = targetLocation.parent.toFile()
            if (!parent.exists())
                parent.mkdirs()

            Files.copy(file.inputStream, targetLocation, StandardCopyOption.REPLACE_EXISTING)
        } catch (ex: IOException) {
            throw FileStorageException("Could not store file $fileName. Please try again!", ex)
        }
    }

    fun setCreationTime(fileName: String, timestamp: Long) {
        val path = fileStorageLocation.resolve(fileName)

        val attributes = Files.getFileAttributeView(
            path,
            BasicFileAttributeView::class.java
        )
        val time = FileTime.fromMillis(timestamp)
        attributes.setTimes(time, time, time)
    }

    fun loadFileAsResource(fileName: String): Resource {
        return try {
            val filePath = fileStorageLocation.resolve(fileName).normalize()
            val resource: Resource = UrlResource(filePath.toUri())
            if (resource.exists()) {
                resource
            } else {
                throw PhotoNotFoundException("File not found $fileName")
            }
        } catch (ex: MalformedURLException) {
            throw PhotoNotFoundException("File not found $fileName", ex)
        }
    }

    fun moveFile(fromFile: String, toFile: String): Boolean {
        val from = fileStorageLocation.resolve(fromFile)
        val to = fileStorageLocation.resolve(toFile)
        return from.toFile().renameTo(to.toFile())
    }

    fun existsFile(fileName: String): Boolean {
        val path = fileStorageLocation.resolve(fileName)
        return path.toFile().exists()
    }

    fun deleteFile(fileName: String): Boolean {
        if (!existsFile(fileName))
            return false

        val targetLocation = fileStorageLocation.resolve(fileName)
        return targetLocation.toFile().delete()
    }

    fun listFiles(folder: String): FileTreeWalk {
        return fileStorageLocation.resolve(folder).toFile().walkBottomUp()
    }
}
