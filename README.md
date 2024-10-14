# Citrix Autolaunch Application

A tiny app for when you're tired of logging into StoreFront.

## Typical Use Case

IoT devices, headless servers, and standalone displays sometimes need to run a remote application through Citrix. A startup process can trigger this app, which automatically logs into StoreFront behind the scenes and launches the requested remote application. Once running, it can also maximize the desired window if you want it to.

## Features

Easy to run and simple to manage, this unit makes your job feel less like work.

**Automatic**

* Set it once and forget it (probably - bugs are being found and evicted)
* Choose whether or not it maximizes your application

**Portable**

* Compile once, and all copies of that executable will share the same hidden encryption key
* Copy it to another computer, and you can even copy the settings file to run without setup
* Executable does not require installation - just copy and execute however you want

**Secure**

* Credentials are stored in an encrypted file
* Keys used for encryption are generated at compile time and stored in the app
* Only interfaces with the domain you feed it

## Build and Run

To compile the app, you'll need rustup. You can then use cargo on the command line, so no need for heavy installations.

1. Install rustup following [the official Rust docs](https://doc.rust-lang.org/cargo/getting-started/installation.html)
2. Clone the repo or download the compressed folder and extract it to your preferred location
3. Navigate to the folder and run the build command: `cargo build -r`
4. Locate `citrix-autolaunch.exe` in the `target` folder and copy to your desired running location

