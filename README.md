# Citrix Autolaunch Application

A tiny app for when you're tired of logging into StoreFront.

## Typical Use Case

IoT devices, headless servers, and standalone displays sometimes need to run a remote application through Citrix. A startup process can trigger this app, which automatically logs into StoreFront behind the scenes and launches the requested remote application. Once running, it can also maximize the desired window if you want it to.

## Features

Easy to run and simple to manage, this unit makes your job feel less like work.

**Automatic**

* Set it once and forget it (probably - bugs are being found and evicted)
* Choose whether or not it maximizes your application
* If the remote program or connection closes, this app will try to re-establish every 5 seconds

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

**Compile**

1. Install rustup following [the official Rust docs](https://doc.rust-lang.org/cargo/getting-started/installation.html)
2. Clone the repo or download the compressed folder and extract it to your preferred location
3. Navigate to the folder and run the build command: `cargo build -r`
4. Locate `citrix-autolaunch.exe` in the `target` folder and copy to your desired running location

**Run**

1. Execute the program
2. Enter the required information
    * StoreFront URL should be entered as `https://my.storefront.url`
    * Application name should be entered exactly as it appears in StoreFront, i.e.: `Google Chrome`
    * Username should be entered exactly as you would type it into StoreFront
    * Password is your password - you remember that, right?
    * Type `y` to maximize, or anything else to turn that feature off
    * If you chose to maximize, enter all or part of the name of the window you want maximized in the next prompt
3. Profit

**Removing Settings**

Should your settings become invalid, there is no option at this time to re-enter them. Delete the `settings.txt` file in the directory and re-run the program to re-enter your settings.

## Requirements

There are very few requirements for this to run...

* Must have execute permissions to the program and directory
* Must have write permissions to create new files in directory for both settings and the ICA file
* Must have access to the StoreFront server or NetScaler on port 443

## Compatibility

This was built and tested on Citrix StoreFront 2402 using Citrix Workspace 2402. Other versions may work. If you have success on another version, let me know!

This has only been tested on Windows 10/11 and MacOS. If you feel like running it on Linux, and it works, let me know!

## Known Issues

* Sometimes repeatedly errors when trying to log in or download the ICA due to network delays (most notable when using MFA or similar)
