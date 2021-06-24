package net.theluckycoder.familyphotos.exceptions

import org.springframework.http.HttpStatus
import org.springframework.web.bind.annotation.ResponseStatus
import java.io.IOException

@ResponseStatus(HttpStatus.NOT_FOUND)
class PhotoNotFoundException : IOException {

    constructor(message: String?) : super(message)
    constructor(message: String?, cause: Throwable?) : super(message, cause)
}
