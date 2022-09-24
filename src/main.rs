use std::path::Path;
use std::io::Error;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::exit;
use argparse::ArgumentParser;
use argparse::List;
use argparse::Store;
use argparse::StoreTrue;
use regex::Regex;
use std::fs::create_dir_all;

mod filesystem;
use filesystem::*;

// gets all files in the folder recursively
fn get_files(path: &OsString, files_out: &mut Vec<OsString>) -> Result<(), Error>
{
   let dir_path = Path::new(path);
   if !dir_path.exists()
   {
        return Err(Error::new(std::io::ErrorKind::NotFound, "Path not found"));
   }

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
fn create_dir_tree(
    paths: &Vec<OsString>,
    output_path: &OsString,
    output_prefix: &OsString,
    index: usize)
-> Result<(), Error>
{
    let mut base_paths: Vec<OsString> = Vec::new();
    for path in paths.iter()
    {
        let index_str: OsString = OsString::from((index+1).to_string());
        let mut group_folder_str: OsString = OsString::new();
        group_folder_str.push(output_prefix.to_owned());
        group_folder_str.push(index_str);

        let full_path = Path::new(&output_path).join(Path::new(&group_folder_str)).join(path);
        let base_path = match full_path.parent() 
        {
            None => { continue; },
            Some(res) => res,
        };

        let base_path_str = base_path.as_os_str().to_os_string();
        if !base_paths.contains(&base_path_str) && !base_path_str.is_empty()
        {
            base_paths.push(base_path_str.to_owned());
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
fn parse_regex(expressions: Vec<String>) -> Result<Vec<Regex>, regex::Error>
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

// create file groups based on regex
fn create_groups(
    regexes: Vec<Regex>,
    paths: Vec<OsString>,
    source_path: &OsString,
    flat: bool) -> Vec<Vec<OsString>>
{
    let mut groups: Vec<Vec<OsString>> = vec![Vec::new(); regexes.len()];
    for path in paths.iter()
    {
        for (index, expr) in regexes.iter().enumerate()
        {
            let dst_path: PathBuf;
            let mut file_path = Path::new(path);
            file_path = match file_path.strip_prefix(&source_path)
            {
                Err(_err) => file_path,
                Ok(res) => res,
            };

            if !flat
            {
                dst_path = file_path.to_path_buf();
            }
            else
            {
                let file_name = match file_path.file_name()
                {
                    None => { continue; },
                    Some(res) => res,
                };
                dst_path = Path::new(file_name).to_path_buf();
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
    groups
}

fn print_group_tree(groups: &Vec<Vec<OsString>>)
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

fn finalize<T: FileHandler>(
    groups: Vec<Vec<OsString>>,
    output_path: OsString,
    output_prefix: OsString,
    source_path: OsString,
    move_files: bool,
    _: T)
-> Result<(), std::io::Error>
{
    for (index, group) in groups.iter().enumerate()
    {
        create_dir_tree(group, &output_path, &output_prefix, index).unwrap();
        for path in group.iter()
        {
            let mut group_folder = OsString::new();
            group_folder.push(&output_prefix);
            let index_str = OsString::from((index+1).to_string());
            group_folder.push(index_str);
            let dst_path = Path::new(&output_path)
                .join(Path::new(&group_folder)
                      .join(Path::new(path)));

            let src_path = Path::new(&source_path).join(path);
            if !move_files
            {
                match T::copy(&src_path, dst_path)
                {
                    Err(err) => { return Err(err); },//panic!("Error copying file {} over to {}!: {}", src_path.to_str().unwrap(), path.to_str().unwrap(), err),
                    Ok(_sz) => {},
                };
            }
            else 
            {
                match T::rename(&src_path, &dst_path)
                {
                    // try copy files and remove them after in case error occurs
                    // std::io::ErrorKind::CrossesDevices not supported in stable Rust
                    Err(_err) =>
                    {
                        match T::copy(&src_path, dst_path)
                        {
                            Err(err) => { return Err(err); },
                            Ok(_sz) => {},
                        };
                        match T::remove_file(&src_path)
                        {
                            Err(err) => { return Err(err); },
                            Ok(()) => {},
                        };
                    }
                    Ok(_sz) => {},
                };
            }
        }
    }
    Ok(())
}

fn main()
{
    let mut source_path: OsString = OsString::new();
    let mut expressions: Vec<String> = Vec::new();
    let mut flat: bool = false;
    let mut output_prefix: OsString = OsString::new();
    let mut output_path: OsString = OsString::new();
    let mut move_files: bool = false;
    let mut print_tree: bool = false;
    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut source_path)
            .add_option(&["-s", "--source"], Store, "Source folder path")
            .required();

        ap.refer(&mut expressions)
            .add_option(&["-e", "--expr"], List,
                "Regular expressions (Expression count is equal to how many folders will be created)")
            .required();

        ap.refer(&mut flat)
            .add_option(&["-f", "--flat"], StoreTrue, "Flatten the structure (folder structure will not be kept)");

        ap.refer(&mut output_prefix)
            .add_option(&["-p", "--prefix"], Store,
                "Prefix of the output folder (default: 1, 2, 3, ...)");

        ap.refer(&mut output_path)
            .add_option(&["-o", "--output"], Store,
                "Output path");

        ap.refer(&mut move_files)
            .add_option(&["-m", "--move"], StoreTrue,
                "Move files instead of copying them");

        ap.refer(&mut print_tree)
            .add_option(&["-t", "--tree"], StoreTrue,
                "Print each group and their matched files without doing anything");

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
    let regexes: Vec<Regex> = match parse_regex(expressions)
    {
        Err(err) => panic!("Failed to parse regex expressions: {}", err),
        Ok(res) => res,
    };

    // put each file in a group matching regex (FIFO order removing matches from the list)
    // if file is already assigned, skip it
    let groups: Vec<Vec<OsString>> = create_groups(regexes, paths, &source_path, flat);

    if print_tree
    {
        print_group_tree(&groups);
        exit(0);
    }

    struct FileHandlerMain;
    impl FileHandler for FileHandlerMain{}
    // create the directory tree and move/copy files over
    match finalize(groups, output_path, output_prefix, source_path, move_files, FileHandlerMain)
    {
        Err(err) => panic!("Error occured while finalizing: {}", err),
        Ok(()) => {},
    };
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    // very hacky way of checking how many times
    // the function has been called
    // must run tests with '-- --test-threads=1' for it to work
    static mut CALLED_COPY: usize = 0;
    static mut CALLED_MOVE: usize = 0;
    static mut CALLED_REMOVE: usize = 0;
    static mut WANT_MOVE_ERR: bool = false;

    struct TestFileHandler;
    impl TestFileHandler
    {
        fn check_files<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), Error>
        {
            static FROM_PATHS: [&'static str; 5] = [
                "source/file1",
                "source/file2",
                "source/file3",
                "source/file4",
                "source/file5"
            ];
            static TO_PATHS: [&'static str; 5] = [
                "output/prefix1/file1",
                "output/prefix1/file2",
                "output/prefix1/file3",
                "output/prefix2/file4",
                "output/prefix2/file5",
            ];
            let from_actual: OsString = from.as_ref().as_os_str().to_os_string();
            let from_actual_str: &str = from_actual.to_str().unwrap();

            let to_actual: OsString = to.as_ref().as_os_str().to_os_string();
            let to_actual_str: &str = to_actual.to_str().unwrap();

            assert!(FROM_PATHS.contains(&from_actual_str));
            if to_actual_str != ""
            {
                assert!(TO_PATHS.contains(&to_actual_str));
                let from_index = FROM_PATHS.iter().position(|i| i == &from_actual_str).unwrap();
                let to_index = TO_PATHS.iter().position(|i| i == &to_actual_str).unwrap();
                assert_eq!(from_index, to_index);
            }
            Ok(())
        }
    }
    impl FileHandler for TestFileHandler
    {
        fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<(), Error>
        {
            unsafe
            {
                CALLED_MOVE += 1;
                if WANT_MOVE_ERR
                {
                    return Err(Error::new(std::io::ErrorKind::Other, ""));
                }
            }
            let _ = TestFileHandler::check_files(from, to);
            Ok(())
        }

        fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> Result<u64, Error>
        {
            unsafe { CALLED_COPY += 1; }
            let _ = TestFileHandler::check_files(from, to);
            Ok(0)
        }

        fn remove_file<P: AsRef<Path>>(from: P) -> Result<(), Error>
        {
            unsafe { CALLED_REMOVE += 1; }
            let _ = TestFileHandler::check_files(from, "");
            Ok(())
        }
    }

    #[test]
    fn test_group()
    {
        let regex_str: Vec<String> = vec![r"\.bin$".to_string(), r"\.d$".to_string()];
        let regexes: Vec<Regex> = parse_regex(regex_str).unwrap();

        let file1: OsString = OsString::from_str("some/path/with/file.bin").unwrap();
        let file2: OsString = OsString::from_str("some/other/path/with/file.bin").unwrap();
        let file3: OsString = OsString::from_str("some/path/with/file.d").unwrap();

        let paths: Vec<OsString> = vec![file1.to_owned(), file2.to_owned(), file3.to_owned()];

        let actual = create_groups(
            regexes,
            paths,
            &OsString::from_str(".").unwrap(),
            false);

        let group1: Vec<OsString> = vec![file1, file2];
        let group2: Vec<OsString> = vec![file3];
        let expected: Vec<Vec<OsString>> = vec![group1, group2];

        assert_eq!(expected, actual);
    }
    
    #[test]
    fn test_finalize_copy()
    {
        let group1: Vec<OsString> = vec![OsString::from("file1"), OsString::from("file2"), OsString::from("file3")];
        let group2: Vec<OsString> = vec![OsString::from("file4"), OsString::from("file5")];
        let groups: Vec<Vec<OsString>> = vec![group1, group2];
        
        unsafe
        {
            CALLED_MOVE = 0;
            CALLED_COPY = 0;
            CALLED_REMOVE = 0;
            WANT_MOVE_ERR = false;
        }
        assert!(matches!(finalize(
            groups,
            OsString::from("output"),
            OsString::from("prefix"),
            OsString::from("source"),
            false,
            TestFileHandler
        ), Ok(())));
        unsafe
        {
            assert_eq!(CALLED_MOVE, 0);
            assert_eq!(CALLED_COPY, 5);
            assert_eq!(CALLED_REMOVE, 0);
        }
    }

    #[test]
    fn test_finalize_move()
    {
        let group1: Vec<OsString> = vec![OsString::from("file1"), OsString::from("file2"), OsString::from("file3")];
        let group2: Vec<OsString> = vec![OsString::from("file4"), OsString::from("file5")];
        let groups: Vec<Vec<OsString>> = vec![group1, group2];
        
        unsafe
        {
            CALLED_MOVE = 0;
            CALLED_COPY = 0;
            CALLED_REMOVE = 0;
            WANT_MOVE_ERR = false;
        }
        assert!(matches!(finalize(
            groups,
            OsString::from("output"),
            OsString::from("prefix"),
            OsString::from("source"),
            true,
            TestFileHandler
        ), Ok(())));
        unsafe
        {
            assert_eq!(CALLED_MOVE, 5);
            assert_eq!(CALLED_COPY, 0);
            assert_eq!(CALLED_REMOVE, 0);
        }
    }

    #[test]
    fn test_finalize_move_failed()
    {
        let group1: Vec<OsString> = vec![OsString::from("file1"), OsString::from("file2"), OsString::from("file3")];
        let group2: Vec<OsString> = vec![OsString::from("file4"), OsString::from("file5")];
        let groups: Vec<Vec<OsString>> = vec![group1, group2];
        
        unsafe
        {
            CALLED_MOVE = 0;
            CALLED_COPY = 0;
            CALLED_REMOVE = 0;
            WANT_MOVE_ERR = true;
        }
        assert!(matches!(finalize(
            groups,
            OsString::from("output"),
            OsString::from("prefix"),
            OsString::from("source"),
            true,
            TestFileHandler
        ), Ok(())));
        unsafe
        {
            assert_eq!(CALLED_MOVE, 5);
            assert_eq!(CALLED_COPY, 5);
            assert_eq!(CALLED_REMOVE, 5);
        }
    }

}
