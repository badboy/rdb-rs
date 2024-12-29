use rstest::*;
use tempfile::TempDir;
use testcontainers_modules::{
    redis::Redis,
    testcontainers::runners::AsyncRunner,
    testcontainers::ImageExt
};
use testcontainers::ContainerAsync;
use testcontainers::core::Mount;
use std::path::Path;
use redis::Client;
use redis::Commands;
use redis;
use rdb;
use tempfile::tempdir;
use std::{fs::File, io::BufReader};

async fn redis_client(major_version: u8, minor_version: u8) -> (Client, TempDir, ContainerAsync<Redis>) {

    let tmp_dir = tempdir().unwrap();
    let container = Redis::default()
        .with_tag(format!("{}.{}-alpine", major_version, minor_version))
        .with_mount(Mount::bind_mount(tmp_dir.path().display().to_string(), "/data"))
        .start()
        .await.expect("Failed to start Redis container");

    let host_ip = container.get_host().await.unwrap();
    let host_port = container.get_host_port_ipv4(6379).await.unwrap();  
    let url = format!("redis://{}:{}", host_ip, host_port);
    let client = Client::open(url).expect("Failed to create Redis client");
    
    (client, tmp_dir, container)
}

fn to_resp_format(command: &str, args: &[&str]) -> String {
    let mut resp = format!("*{}\r\n", args.len() + 1); // +1 for command itself
    resp.push_str(&format!("${}\r\n{}\r\n", command.len(), command));
    for arg in args {
        resp.push_str(&format!("${}\r\n{}\r\n", arg.len(), arg));
    }
    resp
}

async fn execute_commands(conn: &mut redis::Connection, commands: &[(&str, Vec<&str>)]) -> String {
    let mut resp = String::from("*2\r\n$6\r\nSELECT\r\n$1\r\n0\r\n"); // Initial SELECT command
    
    for (cmd, args) in commands {
        // Execute the command
        let mut redis_cmd = redis::cmd(cmd);
        for arg in args {
            redis_cmd.arg(arg);
        }
        redis_cmd.exec(conn).unwrap();

        // Add RESP representation
        resp.push_str(&to_resp_format(cmd, args));
    }
    
    resp
}

fn parse_rdb_to_resp(rdb_path: &Path) -> String {
    let rdb_file = File::open(rdb_path).unwrap();
    let rdb_reader = BufReader::new(rdb_file);
    let tmp_file = tempfile::NamedTempFile::new().unwrap();
    
    rdb::parse(
        rdb_reader, 
        rdb::formatter::Protocol::new(Some(tmp_file.path().to_path_buf())),
        rdb::filter::Simple::new()
    ).unwrap();

    let output = std::fs::read(tmp_file.path()).unwrap();

    String::from_utf8(output).unwrap()
}

fn split_resp_commands(resp: &str) -> Vec<String> {
    // Skip the initial SELECT command
    let mut commands: Vec<String> = resp.split("*")
        .filter(|s| !s.is_empty())
        .map(|s| format!("*{}", s))
        .collect();
    
    // Remove the SELECT command if it exists
    if !commands.is_empty() && commands[0].contains("SELECT") {
        commands.remove(0);
    }
    
    commands
}

#[rstest]
#[case(6, 2)]
#[case(7, 0)]
#[case(7, 2)]
#[case(7, 4)]
#[tokio::test]
async fn test_redis_protocol(#[case] major_version: u8, #[case] minor_version: u8) {
    let (client, tmp_dir, _container) = redis_client(major_version, minor_version).await;
    let mut conn = client.get_connection().unwrap();

    let commands = vec![
        ("SET", vec!["foo", "bar"]),
        ("HSET", vec!["player:1", "name", "John"]),
        ("HSET", vec!["player:1", "age", "25"]),
        ("SADD", vec!["planets", "Earth"]),
        ("SADD", vec!["planets", "Mars"]),
        ("SADD", vec!["planets", "Jupiter"]),
        ("RPUSH", vec!["dogs", "Rex"]),
        ("RPUSH", vec!["dogs", "Bella"]),
        ("RPUSH", vec!["dogs", "Max"]),
    ];

    let expected_resp = execute_commands(&mut conn, &commands).await;
    redis::cmd("SAVE").exec(&mut conn).unwrap();

    let rdb_file = Path::new(&tmp_dir.path()).join("dump.rdb");
    let actual_resp = parse_rdb_to_resp(&rdb_file);

    // Compare commands as unordered sets
    let expected_commands: std::collections::HashSet<_> = split_resp_commands(&expected_resp).into_iter().collect();
    let actual_commands: std::collections::HashSet<_> = split_resp_commands(&actual_resp).into_iter().collect();
    
    assert_eq!(actual_commands, expected_commands);

    let value: String = conn.get("foo").unwrap();
    assert_eq!(value, "bar");
}

