package net.theluckycoder.homeserver.photos

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import net.theluckycoder.homeserver.photos.configs.FileStorageProperties
import net.theluckycoder.homeserver.photos.extensions.LoggerExtensions
import net.theluckycoder.homeserver.photos.repository.PhotoRepository
import net.theluckycoder.homeserver.photos.repository.UserRepository
import net.theluckycoder.homeserver.photos.repository.findByUser
import net.theluckycoder.homeserver.photos.service.FileStorageService
import org.springframework.beans.factory.getBean
import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.context.properties.EnableConfigurationProperties
import org.springframework.boot.runApplication
import org.springframework.context.ConfigurableApplicationContext
import kotlin.time.ExperimentalTime
import kotlin.time.measureTime

@SpringBootApplication
@EnableConfigurationProperties(FileStorageProperties::class)
class PhotosApplication

@OptIn(ExperimentalTime::class)
fun main(args: Array<String>): Unit = runBlocking {
    val applicationContext = runApplication<PhotosApplication>(*args)
    val logger = LoggerExtensions.getLogger<PhotosApplication>()

    coroutineScope {
        launch(Dispatchers.IO) {
            val time = measureTime {
                manageStartData(applicationContext)
            }
            logger.debug("Start Data time needed: $time")
        }
    }
}

private fun manageStartData(applicationContext: ConfigurableApplicationContext) {
    val fileStorageService = applicationContext.getBean<FileStorageService>()
    val userRepository = applicationContext.getBean<UserRepository>()
    val photoRepository = applicationContext.getBean<PhotoRepository>()
    val startData = applicationContext.getBean<StartData>()

    val logger = LoggerExtensions.getLogger<PhotosApplication>()

    if (userRepository.count() == 0L) {
        val initialUsers = startData.getInitialUsers()
        logger.info("Adding Initial Users")
        userRepository.saveAll(initialUsers)
    }

    userRepository.findAll().forEach { user ->
        logger.info("Scanning for user ${user.userName}")
        val foundPhotos = startData.scanForPhotos(user).toMutableList()
        val existingPhotos = photoRepository.findByUser(user)
        val existingPhotosNames = existingPhotos.map { it.fullName }.toSet()

        logger.info("Scanned ${foundPhotos.size} photos in user ${user.userName}")

        // Add any photo that was not already in the database
        foundPhotos.removeAll {
            existingPhotosNames.contains(it.fullName)
        }

        if (foundPhotos.isNotEmpty()) {
            logger.info("Adding ${foundPhotos.size} new photos")
            photoRepository.saveAll(foundPhotos)
        }

        // Remove Photos from the database which are not in the filesystem anymore
        val nonExistentPhotos = existingPhotos.toMutableList()
        nonExistentPhotos.removeAll {
            fileStorageService.existsFile(it.getStorePath(user))
        }

        if (nonExistentPhotos.isNotEmpty()) {
            logger.info("Removing ${nonExistentPhotos.size} non-existent photos")
            photoRepository.deleteAll(nonExistentPhotos)
        }
    }
}
