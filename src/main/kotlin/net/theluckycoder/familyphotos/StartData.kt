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
import net.theluckycoder.familyphotos.repository.PhotoRepository
import net.theluckycoder.familyphotos.service.FileStorageService
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.data.repository.findByIdOrNull
import org.springframework.stereotype.Component
import java.io.File
import java.text.SimpleDateFormat

@Component
class StartData @Autowired constructor(
    securityConfiguration: SecurityConfiguration,
    private val photoRepository: PhotoRepository,
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
        User(displayName = "Rareș", userName = "rares", password = getPassword("tu3sq7ptshyi2"), roles = USER_ROLE),
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

                        val creationTimestamp = getJsonDateTime(file) ?: getRegexDateTime(file)
                        if (creationTimestamp == null) {
                            println("No timestamp: ${file.absolutePath}")
                        }

                        Photo(
                            ownerUserId = user.id,
                            name = file.name,
                            folder = folderName.takeIf { isInSubFolder },
                            timeCreated = creationTimestamp
                                ?: photoRepository.findByName(file.name).firstOrNull()?.timeCreated
                                ?: System.currentTimeMillis(),
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
    private val fileMillisPattern = ".*([0-9]{13}).*".toPattern()
    private val dateHourFormatter = SimpleDateFormat("yyyyMMdd HHmmss")
    private val dateFormatter = SimpleDateFormat("yyyyMMdd")

    private fun getJsonDateTime(file: File): Long? {
        val jsonFileName = file.nameWithoutExtension
            .removeSuffix("-editat").removeSuffix("(1)") + "." + file.extension
        val jsonFile = File(file.parentFile, "$jsonFileName.json")

        if (!jsonFile.exists()) return null

        try {
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
                                timestamp = jParser.valueAsLong * 1000
                        }
                    }
                    "photoTakenTime" -> {
                        jParser.nextToken()
                        while (jParser.nextToken() != JsonToken.END_OBJECT) {
                            if ("timestamp" == jParser.currentName)
                                photoTaken = jParser.valueAsLong * 1000
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

    private fun getRegexDateTime(file: File): Long? {
        try {
            val name = file.nameWithoutExtension
            require(name.length >= 8)

            var matcher = fileDateHourPattern.matcher(name)
            if (matcher.find()) {
                val date = matcher.group(1)
                val hour = matcher.group(2)

                return dateHourFormatter.parse("$date $hour").time
            }

            matcher = fileMillisPattern.matcher(name)
            if (matcher.find()) {
                matcher.group(1)?.toLongOrNull()?.let {
                    return it
                }
            }

            matcher = fileDatePattern.matcher(name)
            if (matcher.find()) {
                val date = matcher.group(1)

                // Please forgive me
                return dateFormatter.parse(date).time.takeIf { it in 2000..2050 }
            }
        } catch (e: Exception) {
        }
        return null
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
