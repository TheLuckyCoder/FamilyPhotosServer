package net.theluckycoder.familyphotos.configs

import org.springframework.boot.context.properties.ConfigurationProperties

@ConfigurationProperties(prefix = "file")
class FileStorageProperties {

    var storageDir: String = ""
}
