package net.theluckycoder.familyphotos.model

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

    /*@OneToMany(
         cascade = [CascadeType.ALL],
         orphanRemoval = true
     )
     @JoinColumn(name = "user_owner_id")
     private val photos = ArrayList<Photo>(64)*/
}

