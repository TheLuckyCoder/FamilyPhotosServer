package net.theluckycoder.homeserver.photos.controller

import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.RestController

@RestController
class HomeController {

    @GetMapping("/")
    fun home() = """<h1>Hello There</h1><br><h2>General Kenobi</h2>
        <div class="tenor-gif-embed" data-postid="4819894" data-share-method="host" data-width="100%" data-aspect-ratio="1.4878048780487805"><a href="https://tenor.com/view/rick-ashtley-never-gonna-give-up-rick-roll-gif-4819894">Rick Ashtley Never Gonna Give Up GIF</a> from <a href="https://tenor.com/search/rickashtley-gifs">Rickashtley GIFs</a></div><script type="text/javascript" async src="https://tenor.com/embed.js"></script>
    """.trimMargin()
}
