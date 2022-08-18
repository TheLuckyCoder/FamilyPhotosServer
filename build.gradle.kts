import org.jetbrains.kotlin.gradle.tasks.KotlinCompile
import org.jetbrains.kotlin.kapt3.base.Kapt.kapt

//import org.springframework.boot.gradle.tasks.bundling.BootBuildImage

plugins {
	id("org.springframework.boot") version "2.7.3"
	id("io.spring.dependency-management") version "1.0.12.RELEASE"

	val kotlinVersion = "1.7.10"
	kotlin("jvm") version kotlinVersion
	kotlin("plugin.spring") version kotlinVersion
	kotlin("plugin.jpa") version kotlinVersion
//	id("org.springframework.experimental.aot") version "0.10.0"
//	id("org.graalvm.buildtools.native") version "0.9.0"

	id("com.github.ben-manes.versions") version "0.42.0"
}

group = "net.theluckycoder"
version = "0.0.1"
java.sourceCompatibility = JavaVersion.VERSION_17

repositories {
	maven { url = uri("https://repo.spring.io/release") }
	mavenCentral()
}

/*springAot {
	removeSpelSupport.set(true)
	removeYamlSupport.set(true)
}*/

dependencies {
	implementation("org.springframework.boot:spring-boot-starter-data-jpa")
	implementation("org.springframework.boot:spring-boot-starter-security")
	implementation("org.springframework.boot:spring-boot-starter-web")

	implementation("com.fasterxml.jackson.module:jackson-module-kotlin")

	kotlin("kotlin-stdlib-jdk8")
	kotlin("kotlin-reflect")
	implementation("org.jetbrains.kotlinx:kotlinx-coroutines-reactor:1.6.4")

	runtimeOnly("com.h2database:h2:1.4.200")

//	testImplementation("org.springframework.boot:spring-boot-starter-test")
//	testImplementation("org.springframework.security:spring-security-test")
}

tasks.withType<KotlinCompile> {
	kotlinOptions {
		freeCompilerArgs = listOf("-Xjsr305=strict", "-Xopt-in=kotlin.RequiresOptIn")
		jvmTarget = "17"
	}
}

tasks.withType<Test> {
	useJUnitPlatform()
}

/*tasks.withType<BootBuildImage> {
	builder = "paketobuildpacks/builder:tiny"
	environment = mapOf("BP_NATIVE_IMAGE" to "true")
	buildpacks = listOf("gcr.io/paketo-buildpacks/java-native-image:5.4.0")
}*/
