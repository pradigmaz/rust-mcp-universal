fn main() {
    #[cfg(windows)]
    println!("cargo:rustc-link-arg-bin=rmu-cli=/STACK:8388608");
}