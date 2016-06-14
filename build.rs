extern crate gcc;

fn main() {
    gcc::compile_library("libgc_clib.a", &["src/heap/gc/clib.c"]);
    
    if cfg!(target_os = "linux") {
        gcc::Config::new()
                     .flag("-lpfm")
                     .flag("-O3")
                     .file("src/common/perf.c")
                     .compile("libgc_perf.a");
    }
}