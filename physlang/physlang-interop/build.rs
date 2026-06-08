fn main() {
    cc::Build::new()
        .file("../../legacy/c/legacy_math.c")
        .compile("physlang_legacy");
}
