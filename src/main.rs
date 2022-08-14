use std::fs::rename;
use std::path::Path;
use std::io::Error;
use std::ffi::OsString;
use std::path::PathBuf;
//use std::process::exit;
use argparse::ArgumentParser;
use argparse::List;
use argparse::Store;
use argparse::StoreTrue;
use regex::Regex;
use std::fs::create_dir_all;
use std::fs::copy;


// gets all files in the folder recursively
fn get_files(path: &OsString, files_out: &mut Vec<OsString>) -> Result<(), Error>
{
   let dir_path = Path::new(path);
   let directory_entries = match dir_path.read_dir()
   {
        Err(_err) => { files_out.push(path.to_owned()); return Ok(()) }, // permission denied errors
                                                                         // are ignored
        Ok(res) => res,
   };
   for entry in directory_entries // if directory has no files it is ignored
   {
        let entry_unwrapped = entry?;
        let info = entry_unwrapped.path();
        let stripped_path = info.as_path();

        let valid_path: PathBuf = match dir_path.join(stripped_path).strip_prefix("./")
        {
            Err(_err) => stripped_path,
            Ok(res) => res,
        }.into();
        
        let valid_path_str = valid_path.as_os_str().to_os_string();

        if info.is_file()
        {
            files_out.push(valid_path_str);
        }
        else
        {
            get_files(&valid_path_str, files_out)?;
        }
   }

   Ok(())
}

// creates a directory tree for all file paths
fn create_dir_tree(paths: &Vec<OsString>) -> Result<(), Error>
{
    let mut base_paths: Vec<OsString> = Vec::new();
    for path in paths.iter()
    {
        let base_path = match Path::new(path).parent()
        {
            None => { continue; },
            Some(res) => res,
        };

        let base_path_str = base_path.as_os_str().to_os_string();
        if !base_paths.contains(&base_path_str) && !base_path_str.is_empty()
        {
            base_paths.push(base_path_str.to_owned()); // TODO: REMOVE to_owned
        }
        else
        {
            continue;
        }
        match create_dir_all(base_path)
        {
            Err(err) => { return Err(err); },
            Ok(_res) => {},
        };
    }
    Ok(())
}

// parse all regex expressions supplied to see if they're valid
fn parse_regex(expressions: &Vec<String>) -> Result<Vec<Regex>, regex::Error>
{
    let mut parsed: Vec<Regex> = Vec::new();
    for expr in expressions.iter()
    {
        let temp = match Regex::new(expr)
        {
            Err(err) => { return Err(err); },
            Ok(res) => res,
        };
        parsed.push(temp);
    }
    Ok(parsed)
}

fn main()
{
    let mut source_path: OsString = OsString::new();
    let mut expressions: Vec<String> = Vec::new();
    let mut flat: bool = false;
    let mut output_prefix: OsString = OsString::new();
    let mut move_files: bool = false;
    let mut print_tree: bool = false;
    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut source_path)
            .add_option(&["-p", "--path"], Store, "Source folder path")
            .required();

        ap.refer(&mut expressions)
            .add_option(&["-e", "--expr"], List,
                "Regular expressions (Expression count is equal to how many folders will be created)")
            .required();

        ap.refer(&mut flat)
            .add_option(&["-f", "--flat"], StoreTrue, "Flatten the structure (folder structure will not be kept)");

        ap.refer(&mut output_prefix)
            .add_option(&["--prefix"], Store,
                "Prefix of the output folder (default: 1, 2, 3, ...)");

        ap.refer(&mut move_files)
            .add_option(&["-m", "--move"], StoreTrue,
                "Move files instead of copying them (only if the source and destination is on the same mount point)");

        ap.refer(&mut print_tree)
            .add_option(&["--tree"], StoreTrue,
                "Print each group and their matched files");

        ap.parse_args_or_exit();
    }

    // get files in the path
    let mut paths: Vec<OsString> = Vec::new();
    match get_files(&source_path, &mut paths)
    {
        Err(err) => panic!("Error getting file list: {}", err),
        Ok(_) => {},
    }

    let file_count = &paths.len();
    println!("Found {} files.", file_count);

    // parse all regex supplied
    let regexes = match parse_regex(&expressions)
    {
        Err(err) => panic!("Failed to parse regex expressions: {}", err),
        Ok(res) => res,
    };

    // put each file in a group matching regex (FIFO order removing matches from the list)
    // if file is already assigned, skip it
    let mut groups: Vec<Vec<OsString>> = vec![Vec::new(); regexes.len()];
    for path in paths.iter()
    {
        for (index, expr) in regexes.iter().enumerate()
        {
            let dst_path: PathBuf;
            let index_str: OsString = OsString::from((index+1).to_string());
            let mut parent_folder_str: OsString = OsString::new();
            parent_folder_str.push(output_prefix.to_owned());
            parent_folder_str.push(index_str);
            let parent_folder = Path::new(&parent_folder_str);
            let mut file_path = Path::new(path);
            file_path = match file_path.strip_prefix(&source_path)
            {
                Err(_err) => file_path,
                Ok(res) => res,
            };

            if !flat
            {
                dst_path = parent_folder.join(file_path);
            }
            else
            {
                let file_name = match file_path.file_name()
                {
                    None => { continue; },
                    Some(res) => res,
                };
                dst_path = parent_folder.join(Path::new(file_name))
            }

            let temp = match path.to_str()
            {
                None => { continue; },
                Some(res) => res,
            };
            if expr.is_match(temp) && !groups[index].contains(path)
            {
                groups[index].push(dst_path.into_os_string());
            }
        }
    }

    if print_tree
    {
        for (index, group) in groups.iter().enumerate()
        {
            println!("Group {}:", index+1);
            for item in group.iter()
            {
                println!("\t{}", item.to_str().unwrap());
            }
        }
    }

    // create the directory tree and move/copy files over
    for (index, group) in groups.iter().enumerate()
    {
        create_dir_tree(group).unwrap();
        for path in group.iter()
        {
            let dst_path = Path::new(path);
            let mut group_folder = OsString::new();
            group_folder.push(&output_prefix);
            let index_str = OsString::from((index+1).to_string());
            group_folder.push(index_str);
            let src_path_stripped = match dst_path.strip_prefix(group_folder)
            {
                Err(_err) => { continue; },
                Ok(res) => res,
            };
            let src_path = Path::new(&source_path).join(src_path_stripped);
            if !move_files
            {
                match copy(&src_path, dst_path)
                {
                    Err(err) => panic!("Error copying file {} over to {}!: {}", src_path.to_str().unwrap(), path.to_str().unwrap(), err),
                    Ok(_sz) => {},
                };
            }
            else 
            {
                match rename(src_path, dst_path)
                {
                    Err(err) => panic!("Error moving file over!: {}", err),
                    Ok(_sz) => {},
                };
            }
        }
    }
}
