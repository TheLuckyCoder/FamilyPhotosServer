package net.theluckycoder.familyphotos

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import net.theluckycoder.familyphotos.configs.FileStorageProperties
import net.theluckycoder.familyphotos.extensions.LoggerExtensions
import net.theluckycoder.familyphotos.repository.PhotoRepository
import net.theluckycoder.familyphotos.repository.UserRepository
import net.theluckycoder.familyphotos.repository.findByUser
import net.theluckycoder.familyphotos.service.FileStorageService
import org.springframework.beans.factory.getBean
import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.autoconfigure.web.embedded.EmbeddedWebServerFactoryCustomizerAutoConfiguration
import org.springframework.boot.context.properties.EnableConfigurationProperties
import org.springframework.boot.runApplication
import org.springframework.context.ConfigurableApplicationContext
import kotlin.time.ExperimentalTime
import kotlin.time.measureTime

@SpringBootApplication(exclude = [EmbeddedWebServerFactoryCustomizerAutoConfiguration::class])
@EnableConfigurationProperties(FileStorageProperties::class)
class FamilyPhotosApplication

@OptIn(ExperimentalTime::class)
fun main(args: Array<String>): Unit = runBlocking {
    val applicationContext = runApplication<FamilyPhotosApplication>(*args)
    val logger = LoggerExtensions.getLogger<FamilyPhotosApplication>()

    coroutineScope {
        launch(Dispatchers.Default) {
            val time = measureTime {
                manageStartData(applicationContext)
            }
            logger.info("Start Data time needed: $time")
        }
    }
}

private suspend fun manageStartData(applicationContext: ConfigurableApplicationContext) {
    val fileStorageService = applicationContext.getBean<FileStorageService>()
    val userRepository = applicationContext.getBean<UserRepository>()
    val photoRepository = applicationContext.getBean<PhotoRepository>()
    val startData = applicationContext.getBean<StartData>()

    val logger = LoggerExtensions.getLogger<FamilyPhotosApplication>()

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
