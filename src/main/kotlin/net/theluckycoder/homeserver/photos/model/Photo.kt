package net.theluckycoder.homeserver.photos.model

import com.fasterxml.jackson.annotation.JsonIgnore
import javax.persistence.Column
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id

@Entity
data class Photo(
    @Id
    @GeneratedValue
    var id: Long = 0,
    @Column(name = "owner")
    val ownerUserId: Long,
    val name: String,
    val timeCreated: Long,
    val fileSize: Long,
    val folder: String? = null, // The category/folder of the file
) {

    val fullName: String
        @JsonIgnore
        get() = buildString {
            if (folder != null)
                append(folder).append('/')
            append(name)
        }

    fun getStorePath(user: User): String {
        require(user.id == ownerUserId) { "Passed user $user does not match owner id $ownerUserId" }
        val str = StringBuilder("photos/").append(user.userName).append('/')

        if (folder != null)
            str.append(folder).append('/')

        str.append(name)
        return str.toString()
    }
}
