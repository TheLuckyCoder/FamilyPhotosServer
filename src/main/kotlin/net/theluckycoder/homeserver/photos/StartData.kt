package net.theluckycoder.homeserver.photos

import com.fasterxml.jackson.core.JsonFactory
import com.fasterxml.jackson.core.JsonParser
import com.fasterxml.jackson.core.JsonToken
import net.theluckycoder.homeserver.photos.configs.SecurityConfiguration
import net.theluckycoder.homeserver.photos.model.Photo
import net.theluckycoder.homeserver.photos.model.User
import net.theluckycoder.homeserver.photos.service.FileStorageService
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

    fun scanForPhotos(user: User): List<Photo> {
        val photos = mutableListOf<Photo>()

        val userFolder = user.userName
        fileStorageService.listFiles("photos/$userFolder").maxDepth(2)
            .filter { it.isFile }
            .filterNot { it.extension == "json" }
            .forEach { file ->
                try {
                    val folderName = file.parentFile.name
                    val isInSubFolder = folderName != userFolder

                    val creationTimestamp = getPhotoCreationTime(file) ?: getAlternativePhotoCreationTime(file) ?: System.currentTimeMillis()

                    photos += Photo(
                        ownerUserId = user.id,
                        name = file.name,
                        folder = folderName.takeIf { isInSubFolder },
                        timeCreated = creationTimestamp,
                        fileSize = file.length()
                    )
                } catch (e: Exception) {
                    e.printStackTrace()
                }
            }

        return photos
    }

    private val fileDatePattern = ".*([0-9]{8}).*([0-9]{6}).*".toPattern()

    private fun getPhotoCreationTime(file: File): Long? {
        val jsonFile = File(file.parentFile, "${file.name}.json")
        try {
            if (!jsonFile.exists()) return null
            val content = jsonFile.readText()
            val jfactory = JsonFactory()
            val jParser: JsonParser = jfactory.createParser(content)

            var timestamp: Long? = null

            while (jParser.nextToken() !== JsonToken.END_OBJECT) {
                val fieldname = jParser.currentName
                if ("creationTime" == fieldname) {
                    jParser.nextToken()
                    while (jParser.nextToken() != JsonToken.END_OBJECT) {
                        if ("timestamp" == jParser.currentName)
                            timestamp = jParser.valueAsLong
                    }
                }
            }
            jParser.close()

            return timestamp
        } catch (e: Exception) {
            println("Failed to parse Json ${jsonFile.absolutePath}")
            return null
        }
    }

    private fun getAlternativePhotoCreationTime(file: File): Long? {
        val path = file.toPath()

        var view: BasicFileAttributes? = null
        try {
            view = Files.getFileAttributeView(path, BasicFileAttributeView::class.java).readAttributes()
        } catch (e: IOException) {
            e.printStackTrace()
        }
        val fileTimeCreation1 = view?.creationTime()?.toMillis()
        val fileTimeCreation2 = view?.lastModifiedTime()?.toMillis()

        //println(date + hour)
        val fileTimeCreation3 = try {
            val dateFormatter = SimpleDateFormat("yyyyMMddHHmmss")
            val name = file.nameWithoutExtension
            require(name.length >= 14)

            val matcher = fileDatePattern.matcher(file.nameWithoutExtension)
            if (matcher.find()) {
                val date = matcher.group(1)
                val hour = matcher.group(2)

                dateFormatter.parse(date + hour).time
            } else null
        } catch (e: Exception) {
            null
        }

        return listOfNotNull(fileTimeCreation1, fileTimeCreation2, fileTimeCreation3)
            .filter { it != 0L }
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
