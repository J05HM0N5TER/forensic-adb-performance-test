use chrono::FixedOffset;
use eyre::{Result, eyre};
use forensic_adb::{AndroidStorageInput, Host, UnixPathBuf};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let host = Host::default();

    let devices = host.devices::<Vec<_>>().await?;
    println!("Found devices: {:?}", devices);

    let device = host
        .device_or_default(Option::<&String>::None, AndroidStorageInput::default())
        .await?;
    println!("Selected device: {:?}", device);
    // FIXME: Set directory to list here, doesn't work on /storage/emulated/0 as a base because of invalid characters, so use a subfolder
    let start_directory = "/storage/emulated/0/Movies";
    let mut current_directory = UnixPathBuf::from(start_directory);

    let now = std::time::Instant::now();
    let dirs = device.list_dir(&current_directory).await?;
    println!(
        "Entries in list_dir {} and took {:?}",
        dirs.iter().len(),
        now.elapsed()
    );
    // Uncomment to view all the entries
    // for dir in dirs {
    //     println!("DIR!: {}", dir.name);
    // }
    let now = std::time::Instant::now();
    let output = device
        .execute_host_shell_command(&format!("ls -1RfpNla {}", &start_directory))
        .await?;
    let mut remote_files = Vec::new();
    for line in output.lines() {
        if line.starts_with(".") || line.starts_with("/") {
            current_directory = UnixPathBuf::from(line);
        } else if line.len() > 0 {
            const FILE_SIZE_INDEX: usize = 1;
            const MODIFIED_DATETIME_DATE_INDEX: usize = 2;
            const MODIFIED_DATETIME_TIME_INDEX: usize = 3;
            const FILE_NAME_START_INDEX: usize = 4;
            const NUMBER_OF_EXPECTED_PARTS_IN_FILE_LINE: usize = 5;
            let split: Vec<String> = line
                .split(" ")
                .map(|s| s.to_string())
                // Has an empty element that we need to filter out
                .filter(|s| s.len() > 0)
                .collect();
            if split.len() < NUMBER_OF_EXPECTED_PARTS_IN_FILE_LINE {
                return Err(eyre!("Not enough parts in line"));
            }

            let rfc3339_formatted_last_modified_date = format!(
                "{}T{}:00+00:00",
                split[MODIFIED_DATETIME_DATE_INDEX], split[MODIFIED_DATETIME_TIME_INDEX],
            );

            let size = &split[FILE_SIZE_INDEX].parse::<u64>()?;
            let size = size.clone();
            let modified_time =
                chrono::DateTime::parse_from_rfc3339(&rfc3339_formatted_last_modified_date)?;
            let file_name = split[FILE_NAME_START_INDEX..].join(" ");
            let path = current_directory.join(file_name);
            remote_files.push(File {
                modified_datetime: modified_time,
                size,
                path,
            });
        }
    }
    // Uncomment to view all the entries
    // for remote_file in remote_files {
    //     println!("{:?}", remote_file);
    // }
    println!(
        "Entries in manual ls parse: {} and took {:?}",
        remote_files.len(),
        now.elapsed()
    );
    Ok(())
}

#[derive(Debug)]
struct File {
    pub size: u64,
    pub path: UnixPathBuf,
    pub modified_datetime: chrono::DateTime<FixedOffset>,
}
