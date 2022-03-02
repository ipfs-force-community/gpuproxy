use std::env;
use std::fs::{create_dir_all, rename, File};
use std::io::{self, copy, stderr, stdout, Read, Stdout, Write};
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

use anyhow::{ensure, Context, Result};
use filecoin_proofs::param::{get_digest_for_file_within_cache, get_full_path_for_file_within_cache, has_extension};
use flate2::read::GzDecoder;
use humansize::{file_size_opts, FileSize};
use log::{error, info, trace, warn};
use pbr::{ProgressBar, Units};
use reqwest::{blocking::Client, header, Proxy, Url};
use storage_proofs_core::parameter_cache::{
    parameter_cache_dir, parameter_cache_dir_name, ParameterMap, GROTH_PARAMETER_EXT, VERIFYING_KEY_EXT,
};
use tar::Archive;

const DEFAULT_JSON: &str = include_str!("./parameters.json");
const DEFAULT_IPGET_VERSION: &str = "v0.6.0";

#[inline]
fn get_ipget_dir(version: &str) -> String {
    format!("/var/tmp/ipget-{}", version)
}

#[inline]
fn get_ipget_path(version: &str) -> String {
    format!("{}/ipget/ipget", get_ipget_dir(version))
}

/// Reader with progress bar.
struct FetchProgress<R> {
    reader: R,
    progress_bar: ProgressBar<Stdout>,
}

impl<R: Read> Read for FetchProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf).map(|n| {
            self.progress_bar.add(n as u64);
            n
        })
    }
}

impl<R: Read> FetchProgress<R> {
    fn new(reader: R, size: u64) -> Self {
        let mut progress_bar = ProgressBar::new(size);
        progress_bar.set_units(Units::Bytes);
        FetchProgress { reader, progress_bar }
    }
}

/// Download a version of ipget.
fn download_ipget(version: &str, verbose: bool) -> Result<()> {
    info!("downloading ipget");
    let (os, ext) = if cfg!(target_os = "macos") {
        ("darwin", "tar.gz")
    } else if cfg!(target_os = "windows") {
        // TODO: enable Windows by adding support for .zip files.
        // ("windows", "zip")
        unimplemented!("paramfetch does not currently support Windows/.zip downloads");
    } else {
        ("linux", "tar.gz")
    };

    // Request ipget file.
    let url = Url::parse(&format!("https://dist.ipfs.io/ipget/{}/ipget_{}_{}-amd64.{}", version, version, os, ext,))?;
    trace!("making GET request: {}", url.as_str());
    let client = Client::builder().build()?;
    let mut resp = client.get(url).send()?;
    trace!("received GET response");
    if !resp.status().is_success() {
        error!("non-200 response status:\n{:?}\nexiting", resp);
        exit(1);
    }

    let size: Option<u64> = resp.headers().get(header::CONTENT_LENGTH).and_then(|val| val.to_str().unwrap().parse().ok());

    match size {
        Some(size) => trace!("content-length: {}", size),
        None => trace!("unable to parse content-length: {:?}", resp.headers().get(header::CONTENT_LENGTH),),
    };

    // Write downloaded file.
    let write_path = format!("{}.{}", get_ipget_dir(version), ext);
    trace!("writing downloaded file to: {}", write_path);
    let mut writer = File::create(&write_path).expect("failed to create file");
    if verbose {
        if let Some(size) = size {
            let mut resp_with_progress = FetchProgress::new(resp, size);
            copy(&mut resp_with_progress, &mut writer).expect("failed to write download to file");
        }
    } else {
        copy(&mut resp, &mut writer).expect("failed to write download to file");
    }
    drop(writer);

    // Unzip and unarchive downloaded file.
    let reader = File::open(&write_path).expect("failed to open downloaded tar file");
    if ext == "tar.gz" {
        trace!("unzipping and unarchiving downloaded file");
        let unzipper = GzDecoder::new(reader);
        let mut unarchiver = Archive::new(unzipper);
        unarchiver.unpack(get_ipget_dir(version)).expect("failed to unzip and unarchive");
    } else {
        unimplemented!("unzip is not yet supported");
    }
    info!("successfully downloaded ipget binary: {}", get_ipget_path(version),);

    Ok(())
}

/// Check which files are outdated (or no not exist).
fn get_filenames_requiring_download(parameter_map: &ParameterMap, selected_filenames: Vec<String>) -> Vec<String> {
    selected_filenames
        .into_iter()
        .filter(|filename| {
            trace!("determining if file is out of date: {}", filename);
            let path = get_full_path_for_file_within_cache(filename);
            if !path.exists() {
                trace!("file not found, marking for download");
                return true;
            };
            trace!("params file found");
            let calculated_digest = match get_digest_for_file_within_cache(filename) {
                Ok(digest) => digest,
                Err(e) => {
                    warn!("failed to hash file {}, marking for download", e);
                    return true;
                }
            };
            let expected_digest = &parameter_map[filename].digest;
            if &calculated_digest == expected_digest {
                trace!("file is up to date");
                false
            } else {
                trace!("file has unexpected digest, marking for download");
                let new_filename = format!("{}-invalid-digest", filename);
                let new_path = path.with_file_name(new_filename);
                trace!("moving invalid params to: {}", new_path.display());
                rename(path, new_path).expect("failed to move file");
                true
            }
        })
        .collect()
}

fn download_file_with_ipget(cid: &str, path: &Path, ipget_path: &Path, ipget_args: &Option<String>, verbose: bool) -> Result<()> {
    let mut args = vec![cid, "-o", path.to_str().unwrap()];
    if let Some(ipget_args) = ipget_args {
        args.extend(ipget_args.split_whitespace());
    }
    trace!("spawning subprocess: {} {}", ipget_path.display(), args.join(" "));
    let output = Command::new(ipget_path.as_os_str())
        .args(&args)
        .output()
        .with_context(|| "failed to spawn ipget subprocess")?;
    if verbose {
        stdout().write_all(&output.stdout).with_context(|| "failed to write ipget's stdout")?;
        stderr().write_all(&output.stderr).with_context(|| "failed to write ipget's stderr")?;
    }
    ensure!(output.status.success(), "ipget returned non-zero exit code");
    Ok(())
}

pub fn download_sector_size(sector_sizes: Option<Vec<u64>>) {
    // Parse parameters.json file.
    let parameter_map: ParameterMap = serde_json::from_str(DEFAULT_JSON)
        .map_err(|e| {
            error!("failed to parse built-in json, exiting\n{:?}", e);
            exit(1);
        })
        .unwrap();

    let mut filenames: Vec<String> = parameter_map.keys().cloned().collect();
    trace!("json contains {} files", filenames.len());

    // Filter out unwanted sector sizes from params files (.params files only, leave verifying-key
    // files).
    if let Some(ref sector_sizes) = sector_sizes {
        filenames.retain(|filename| {
            let remove = has_extension(filename, VERIFYING_KEY_EXT)
                || (has_extension(filename, GROTH_PARAMETER_EXT) && !sector_sizes.contains(&parameter_map[filename].sector_size));
            if remove {
                let human_size = parameter_map[filename].sector_size.file_size(file_size_opts::BINARY).unwrap();
                trace!("ignoring file: {} ({})", filename, human_size);
            }
            !remove
        });
    }

    // Determine which files are outdated.
    filenames = get_filenames_requiring_download(&parameter_map, filenames);
    if filenames.is_empty() {
        info!("no outdated files, exiting");
        return;
    }

    info!("{} files to be downloaded: {:?}", filenames.len(), filenames);

    if filenames.is_empty() {
        info!("no files to download, exiting");
        return;
    }

    let tmp_path = get_ipget_path(DEFAULT_IPGET_VERSION);
    let ipget_path = PathBuf::from(&tmp_path);
    if !ipget_path.exists() {
        info!("ipget binary not found: {}", ipget_path.display());
        download_ipget(DEFAULT_IPGET_VERSION, true).expect("ipget download failed");
    }
    trace!("using ipget binary: {}", ipget_path.display());

    trace!("creating param cache dir(s) if they don't exist");
    create_dir_all(parameter_cache_dir()).expect("failed to create param cache dir");

    loop {
        for filename in &filenames {
            info!("downloading params file with ipget: {}", filename);
            let path = get_full_path_for_file_within_cache(filename);
            match download_file_with_ipget(&parameter_map[filename].cid, &path, &ipget_path, &None, true) {
                Ok(_) => info!("finished downloading params file"),
                Err(e) => warn!("failed to download params file: {}", e),
            };
        }
        filenames = get_filenames_requiring_download(&parameter_map, filenames);
        if filenames.is_empty() {
            info!("succesfully updated all files, exiting");
            return;
        }
        warn!("{} files failed to be fetched: {:?}", filenames.len(), filenames);
    }
}
