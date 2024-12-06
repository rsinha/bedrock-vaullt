use clap::{Command, Arg, value_parser};
use std::{fs, path::PathBuf};

const VAULT_DIR_NAME: &str = ".bedrock";
const VAULT_KEM_CTXT_FILENAME : &str = "kem_ctxt";
const VAULT_DEM_CTXT_FILENAME : &str = "dem_ctxt";

fn main() {
    let matches = Command::new("Vault")
        .version("1.0")
        .about("A simple vault application that allows you to store and retrieve secrets")
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .help("Sets the operation mode")
                .value_parser(["reload", "init"])
                .required(true)
        )
        .arg(
            Arg::new("pincode")
                .short('p')
                .long("pincode")
                .help("6-digit numeric pincode")
                .required(true)
                .value_parser(value_parser!(String))
        )
        .arg(
            Arg::new("secret")
                .short('s')
                .long("secret")
                .help("Secret of any length")
                .value_parser(value_parser!(String))
        )
        .get_matches();

    // Get the values of the arguments
    let mode = matches.get_one::<String>("mode").unwrap();
    let pin = matches.get_one::<String>("pincode").unwrap();

    let (kem_path, dem_path) = get_vault_directory();

    // Process based on the mode
    match mode.as_str() {
        "reload" => {
            println!("Reloading secret from vault using pincode {}", pin);
            let client = bedrock_vault::BedrockClient::new(
                "https://zkbricks-vault-worker.rohit-fd0.workers.dev/decrypt", 
                &kem_path,
                &dem_path
            );
            let recovered_secret = client.recover(pin.as_bytes()).unwrap();
            println!("Recovered secret: {:?}", String::from_utf8(recovered_secret).unwrap());
        },
        "init" => {
            let secret = matches.get_one::<String>("secret");
            if secret.is_none() {
                panic!("ERROR: secret is required for initialization");
            }

            println!("Creating a vault with pincode {}", pin);
            let client = bedrock_vault::BedrockClient::new(
                "https://zkbricks-vault-worker.rohit-fd0.workers.dev/decrypt", 
                &kem_path,
                &dem_path
            );
            let password = pin.as_bytes();
            let secret = secret.unwrap().as_bytes();
            client.initialize(password, secret).unwrap();
        },
        _ => unreachable!(), // This won't happen due to value_parser restriction
    }
}

pub fn get_vault_directory() -> (PathBuf, PathBuf) {
        // Get the user's home directory
        let home_dir = directories::BaseDirs::new().unwrap().home_dir().to_path_buf();
    
        // Define the app directory
        let app_dir = home_dir.join(VAULT_DIR_NAME);
    
        // Create the directory if it doesn't exist
        if !app_dir.exists() {
            match fs::create_dir_all(&app_dir) {
                Ok(_) => println!("Directory created successfully at: {:?}", app_dir),
                Err(e) => eprintln!("Failed to create directory: {}", e),
            }
        } else {
            println!("Directory already exists at: {:?}", app_dir);
        }

        let kem_path = app_dir.join(VAULT_KEM_CTXT_FILENAME);
        println!("KEM ciphertext location: {:?}", kem_path);
        let dem_path = app_dir.join(VAULT_DEM_CTXT_FILENAME);
        println!("DEM ciphertext location: {:?}", dem_path);
        
        return (kem_path, dem_path);
}