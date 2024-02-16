use flate2::read::GzDecoder;
use std::env;
use std::fs::File;
use std::path::PathBuf;
use tar::Archive;

mod setup;

fn unpack(source: PathBuf, destination: PathBuf) -> Result<(), std::io::Error> {
    let f = File::open(source).unwrap();
    let gz = GzDecoder::new(f);
    let mut ar = Archive::new(gz);
    ar.unpack(destination)?;
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let destination = PathBuf::from("/destination");
    let in_disk = PathBuf::from(&args[1]);
    let out_disk = PathBuf::from(&args[2]);
    let filesystem = PathBuf::from(&args[3]);

    println!("Setting up environment");
    setup::setup_environment().unwrap();
    println!("Mounting filesystem");
    setup::mount(Some(out_disk), destination.clone(), filesystem).unwrap();
    println!("Unpacking filesystem");
    let res = unpack(in_disk, destination);
    match res {
        Err(e) => println!("{:#?}", e),
        Ok(()) => (),
    };

    setup::sync();
}
