plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("maven-publish")
}

android {
    namespace = "com.elara.sdk"
    compileSdk = 34

    defaultConfig {
        minSdk = 24
    }

    sourceSets["main"].java.srcDirs("src/main/kotlin")
    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
}

val repoRoot = rootProject.projectDir.parentFile.parentFile
val jniLibsDir = layout.projectDirectory.dir("src/main/jniLibs")

tasks.register<Exec>("cargoBuild") {
    val outputDir = jniLibsDir.asFile
    commandLine(
        "cargo",
        "ndk",
        "-o",
        outputDir.absolutePath,
        "-t",
        "armeabi-v7a",
        "-t",
        "arm64-v8a",
        "-t",
        "x86",
        "-t",
        "x86_64",
        "build",
        "-p",
        "elara-ffi",
        "--release"
    )
    workingDir = repoRoot
    outputs.dir(outputDir)
}

dependencies {
    // ML Kit Face Detection (for ElaraBeautyFilter)
    implementation("com.google.mlkit:face-detection:16.1.5")
}

tasks.named("preBuild") {
    dependsOn("cargoBuild")
}

publishing {
    publications {
        create<MavenPublication>("release") {
            groupId = "com.elara"
            artifactId = "elara-sdk"
            version = "0.1.0"
            afterEvaluate {
                from(components["release"])
            }
        }
    }
    repositories {
        mavenLocal()
    }
}
