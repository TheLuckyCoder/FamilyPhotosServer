package net.theluckycoder.familyphotos.model

import com.fasterxml.jackson.annotation.JsonIgnore
import net.theluckycoder.familyphotos.utils.IdGenerator
import org.hibernate.Hibernate
import javax.persistence.Column
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id
import javax.persistence.Transient

@Entity
data class Photo(
    @Id
    val id: Long = IdGenerator.get(),
    @Column(name = "owner")
    val ownerUserId: Long,
    val name: String,
    val timeCreated: Long,
    val fileSize: Long,
    val folder: String? = null, // The category/folder of the file
) {

    val fullName: String
        @JsonIgnore
        @Transient
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

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other == null || Hibernate.getClass(this) != Hibernate.getClass(other)) return false
        other as Photo

        return id == other.id
    }

    override fun hashCode(): Int = id.toInt()

    override fun toString(): String =
        this::class.simpleName + "(id = $id, ownerUserId = $ownerUserId, name = $name, timeCreated = $timeCreated, fileSize = $fileSize, folder = $folder)"
}
