package net.theluckycoder.homeserver.photos.repository

import net.theluckycoder.homeserver.photos.model.Photo
import net.theluckycoder.homeserver.photos.model.User
import org.springframework.data.jpa.repository.Query
import org.springframework.data.repository.CrudRepository
import org.springframework.data.repository.query.Param
import org.springframework.stereotype.Repository

@Repository
interface PhotoRepository : CrudRepository<Photo, Long> {

    @Query("FROM Photo photo WHERE photo.ownerUserId=:userId ORDER BY photo.timeCreated DESC")
    fun findByUser(@Param("userId") userId: Long): Iterable<Photo>

}

fun PhotoRepository.findByUser(user: User): Iterable<Photo> = findByUser(user.id)
