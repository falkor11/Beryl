fn main() {
    // Tell cargo to pass the linker script to the linker..
    println!("cargo:rustc-link-arg=-Tlinker.ld");
    // ..and to re-run if it changes.
    println!("cargo:rerun-if-changed=linker.ld");
    println!("cargo:rerun-if-changed=src/handlers.asm");

    {
        let mut build = nasm_rs::Build::new();

        build
            .file("src/handlers.asm")
            .flag("-felf64")
            .target("x86_64-unknown-none");

        build
            .compile("handlers")
            .expect("failed to compile assembly: skill issue");

        println!("cargo:rustc-link-lib=static=handlers");
    }
}
