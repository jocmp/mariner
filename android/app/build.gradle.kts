import com.android.build.api.variant.ApplicationAndroidComponentsExtension

plugins {
  alias(libs.plugins.android.application)
  alias(libs.plugins.compose.compiler)
  alias(libs.plugins.kotlin.serialization)
}

android {
  namespace = "com.jocmp.mariner"
  compileSdk = 37
  ndkVersion = "28.2.13676358"
  defaultConfig {
    applicationId = "com.jocmp.mariner"
    minSdk = 31
    targetSdk = 37
    versionCode = 1
    versionName = "2026.11.1001"
  }

  buildTypes {
    release {
      isMinifyEnabled = false
      proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
    }
  }
  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
  buildFeatures {
    compose = true
    aidl = false
    buildConfig = false
    shaders = false
  }

  packaging {
    resources {
      excludes += "/META-INF/{AL2.0,LGPL2.1}"
    }
  }
}

kotlin {
  jvmToolchain(17)
}

dependencies {
  val composeBom = platform(libs.androidx.compose.bom)
  implementation(composeBom)
  androidTestImplementation(composeBom)

  // Core Android dependencies
  implementation(libs.androidx.core.ktx)
  implementation(libs.androidx.lifecycle.runtime.ktx)
  implementation(libs.androidx.activity.compose)

  // Arch Components
  implementation(libs.androidx.lifecycle.runtime.compose)
  implementation(libs.androidx.lifecycle.viewmodel.compose)

  // Compose
  implementation(libs.androidx.compose.ui)
  implementation(libs.androidx.compose.ui.tooling.preview)
  implementation(libs.androidx.compose.material3)
  // Tooling
  debugImplementation(libs.androidx.compose.ui.tooling)
  // Instrumented tests
  androidTestImplementation(libs.androidx.compose.ui.test.junit4)
  debugImplementation(libs.androidx.compose.ui.test.manifest)

  // Local tests: jUnit, coroutines, Android runner
  testImplementation(libs.junit)
  testImplementation(libs.kotlinx.coroutines.test)

  // Instrumented tests: jUnit rules and runners
  androidTestImplementation(libs.androidx.test.core)
  androidTestImplementation(libs.androidx.test.ext.junit)
  androidTestImplementation(libs.androidx.test.runner)
  androidTestImplementation(libs.androidx.test.espresso.core)

  // Navigation
  implementation(libs.androidx.navigation3.ui)
  implementation(libs.androidx.navigation3.runtime)
  implementation(libs.androidx.lifecycle.viewmodel.navigation3)
  implementation(libs.androidx.compose.material3.adaptive.navigation3)
  implementation(libs.androidx.compose.material.icons.extended)

  // The Rust core, through its generated bindings
  implementation(libs.jna) { artifact { type = "aar" } }
  implementation(libs.kotlinx.coroutines.android)

  implementation(libs.okhttp)
  implementation(libs.coil.compose)
  implementation(libs.coil.network.okhttp)
  implementation(libs.coil.gif)
  implementation(libs.androidx.work.runtime.ktx)
  implementation(project(":swiperefresh"))
}

// The Rust core (crates/mariner-ffi) is built with cargo-ndk into generated
// jniLibs, and its Kotlin bindings generated with UniFFI, for every build.
// Cargo decides what needs rebuilding, so both tasks always run.

/** The repository root, holding the Cargo workspace. */
val workspace: File = rootDir.parentFile

/** Cargo from rustup, for Gradle daemons started without the shell's PATH. */
val cargo: String =
  File(System.getProperty("user.home"), ".cargo/bin/cargo").takeIf { it.exists() }?.path ?: "cargo"

abstract class CargoNdk : Exec() {
  @get:OutputDirectory abstract val outputDir: DirectoryProperty
}

abstract class UniffiBindings : Exec() {
  @get:OutputDirectory abstract val outputDir: DirectoryProperty
}

val ndkDirectory =
  extensions.getByType<ApplicationAndroidComponentsExtension>().sdkComponents.ndkDirectory

/**
 * Debug builds are arm64 only (phones and Apple silicon emulators), built with
 * Cargo's release profile so they're quick to build and fast to run. Release
 * builds cover each ABI with the mobile profile (LTO).
 */
fun cargoNdk(name: String, abis: List<String>, profile: String) =
  tasks.register<CargoNdk>(name) {
    group = "rust"
    description = "Builds the Rust core for ${abis.joinToString()}"
    outputDir.set(layout.buildDirectory.dir("generated/rust/$name/jniLibs"))
    outputs.upToDateWhen { false }
    workingDir = workspace
    environment("ANDROID_NDK_HOME", ndkDirectory.get().asFile.path)
    val targets = abis.flatMap { listOf("-t", it) }
    commandLine(
      listOf(cargo, "ndk") + targets +
        listOf("--platform", "31", "-o", outputDir.get().asFile.path,
          "build", "--locked", "--profile", profile, "-p", "mariner-ffi")
    )
  }

val cargoNdkDebug = cargoNdk("cargoNdkDebug", listOf("arm64-v8a"), "release")
val cargoNdkRelease =
  cargoNdk("cargoNdkRelease", listOf("arm64-v8a", "armeabi-v7a", "x86_64"), "mobile")

val uniffiBindings =
  tasks.register<UniffiBindings>("uniffiBindings") {
    group = "rust"
    description = "Generates the Kotlin bindings for the Rust core"
    outputDir.set(layout.buildDirectory.dir("generated/rust/kotlin"))
    outputs.upToDateWhen { false }
    workingDir = workspace
    val library = workspace.resolve("target/debug/libmariner_ffi." +
      if (System.getProperty("os.name").startsWith("Mac")) "dylib" else "so")
    commandLine(
      "sh", "-c",
      "'$cargo' build --locked -p mariner-ffi && '$cargo' run --locked -p mariner-bindgen -- " +
        "generate --library '$library' --language kotlin --no-format " +
        "--out-dir '${outputDir.get().asFile.path}'",
    )
  }

androidComponents {
  onVariants { variant ->
    val native = if (variant.buildType == "release") cargoNdkRelease else cargoNdkDebug
    variant.sources.jniLibs?.addGeneratedSourceDirectory(native, CargoNdk::outputDir)
    variant.sources.kotlin?.addGeneratedSourceDirectory(uniffiBindings, UniffiBindings::outputDir)
  }
}
