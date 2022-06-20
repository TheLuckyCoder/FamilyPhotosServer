package net.theluckycoder.familyphotos.model

import org.hibernate.Hibernate
import javax.persistence.Entity
import javax.persistence.GeneratedValue
import javax.persistence.Id

@Entity
data class User(
    @Id
    @GeneratedValue
    var id: Long = 0,
    val displayName: String,
    val userName: String,
    val password: String,
    val active: Boolean = true,
    val roles: String
) {

    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other == null || Hibernate.getClass(this) != Hibernate.getClass(other)) return false
        other as User

        return id == other.id
    }

    override fun hashCode(): Int = javaClass.hashCode()

    @Override
    override fun toString(): String {
        return this::class.simpleName + "(id = $id , displayName = $displayName , userName = $userName , password = $password , active = $active , roles = $roles )"
    }

    /*@OneToMany(
         cascade = [CascadeType.ALL],
         orphanRemoval = true
     )
     @JoinColumn(name = "user_owner_id")
     private val photos = ArrayList<Photo>(64)*/
}

