package net.theluckycoder.familyphotos

import com.fasterxml.jackson.core.JsonFactory
import com.fasterxml.jackson.core.JsonParser
import com.fasterxml.jackson.core.JsonToken
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.withTimeout
import net.theluckycoder.familyphotos.configs.SecurityConfiguration
import net.theluckycoder.familyphotos.extensions.asyncForEach
import net.theluckycoder.familyphotos.model.Photo
import net.theluckycoder.familyphotos.model.User
import net.theluckycoder.familyphotos.service.FileStorageService
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.stereotype.Component
import java.io.File
import java.io.IOException
import java.nio.file.Files
import java.nio.file.attribute.BasicFileAttributeView
import java.nio.file.attribute.BasicFileAttributes
import java.text.SimpleDateFormat

@Component
class StartData @Autowired constructor(
    securityConfiguration: SecurityConfiguration,
    private val fileStorageService: FileStorageService
) {

    private val encoder = securityConfiguration.getPasswordEncoder()

    fun getInitialUsers() = listOf(
        User(
            displayName = "Public",
            userName = PUBLIC_USERNAME,
            password = getPassword("hnuhjaw13odja"),
            roles = PUBLIC_ROLE
        ),
        User(displayName = "Răzvan", userName = "razvan", password = getPassword("938pm7hqryrgo"), roles = USER_ROLE),
        User(displayName = "Rares", userName = "rares", password = getPassword("tu3sq7ptshyi2"), roles = USER_ROLE),
        User(displayName = "Anda", userName = "anda", password = getPassword("7nb9nx84t6n47"), roles = USER_ROLE),
        User(displayName = "Adonis", userName = "adonis", password = getPassword("qpph2t6qzptp9"), roles = USER_ROLE),
    )

    suspend fun scanForPhotos(user: User): List<Photo> = coroutineScope {
        val userFolder = user.userName

        fileStorageService.listFiles("photos/$userFolder").maxDepth(2)
            .filter { it.isFile }
            .filterNot { it.extension == "json" }
            .asyncForEach(this) { file ->
                try {
                    withTimeout(8000) {
                        val folderName = file.parentFile.name
                        val isInSubFolder = folderName != userFolder

                        val creationTimestamp = getPhotoCreationTime(file) ?: getAlternativePhotoCreationTime(file)
                        if (creationTimestamp == null) {
                            println("No timestamp: ${file.absolutePath}")
                        }

                        Photo(
                            ownerUserId = user.id,
                            name = file.name,
                            folder = folderName.takeIf { isInSubFolder },
                            timeCreated = creationTimestamp ?: System.currentTimeMillis(),
                            fileSize = file.length()
                        )
                    }
                } catch (e: TimeoutCancellationException) {
                    println("Time out: ${file.absolutePath}")
                    null
                } catch (e: Exception) {
                    e.printStackTrace()
                    null
                }
            }
            .toList()
            .awaitAll()
            .filterNotNull()
    }

    private val fileDateHourPattern = ".*([0-9]{8}).*([0-9]{6}).*".toPattern()
    private val fileDatePattern = ".*([0-9]{8}).*".toPattern()
    private val dateHourFormatter = SimpleDateFormat("yyyyMMddHHmmss")
    private val dateFormatter = SimpleDateFormat("yyyyMMdd")

    private fun getPhotoCreationTime(file: File): Long? {
        val jsonFile = File(file.parentFile, "${file.name}.json")
        try {
            if (!jsonFile.exists()) return null
            val content = jsonFile.readText()
            val jfactory = JsonFactory()
            val jParser: JsonParser = jfactory.createParser(content)

            var timestamp: Long? = null
            var photoTaken: Long? = null

            while (jParser.nextToken() !== JsonToken.END_OBJECT) {
                when (jParser.currentName) {
                    "creationTime" -> {
                        jParser.nextToken()
                        while (jParser.nextToken() != JsonToken.END_OBJECT) {
                            if ("timestamp" == jParser.currentName)
                                timestamp = jParser.valueAsLong
                        }
                    }
                    "photoTakenTime" -> {
                        jParser.nextToken()
                        while (jParser.nextToken() != JsonToken.END_OBJECT) {
                            if ("timestamp" == jParser.currentName)
                                photoTaken = jParser.valueAsLong
                        }
                    }
                    else -> if (timestamp != null && photoTaken != null) {
                        break
                    }
                }
            }
            jParser.close()

            return listOfNotNull(timestamp, photoTaken).minOrNull()
        } catch (e: Exception) {
            println("Failed to parse Json ${jsonFile.absolutePath}")
            return null
        }
    }

    private fun getAlternativePhotoCreationTime(file: File): Long? {
        val path = file.toPath()

        try {
            val name = file.nameWithoutExtension
            require(name.length >= 14)

            var matcher = fileDateHourPattern.matcher(name)
            if (matcher.find()) {
                val date = matcher.group(1)
                val hour = matcher.group(2)

                dateHourFormatter.parse(date + hour).time
            } else {
                matcher = fileDatePattern.matcher(name)
                if (matcher.find()) {
                    val date = matcher.group(1)

                    dateFormatter.parse(date).time
                } else null
            }
        } catch (e: Exception) {
            null
        }?.let {
            return it
        }

        var view: BasicFileAttributes? = null
        try {
            view = Files.getFileAttributeView(path, BasicFileAttributeView::class.java).readAttributes()
        } catch (e: IOException) {
            e.printStackTrace()
        }
        val fileTimeCreation1 = view?.creationTime()?.toMillis()
        val fileTimeCreation2 = view?.lastModifiedTime()?.toMillis()

        return listOfNotNull(fileTimeCreation1, fileTimeCreation2)
            .filterNot { it <= 1000000000000L }
            .minByOrNull { it }
    }

    private fun getPassword(password: String): String {
        return encoder.encode(password)
    }

    companion object {
        private const val USER_ROLE = SecurityConfiguration.Role.USER
        private const val PUBLIC_ROLE = SecurityConfiguration.Role.PUBLIC

        const val PUBLIC_USERNAME = "public"
    }
}
