//! Test doubles: a one-shot loopback HTTP server and a scratch workspace with fake tools.

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;

use crate::repo::Workspace;

/// Serves one canned HTTP response on loopback and returns the raw request it received.
pub(crate) fn serve(reply: impl Into<String>) -> (String, JoinHandle<String>) {
    let reply = reply.into();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream);
        let mut head = String::new();
        let mut length = 0_usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().unwrap();
            }
            head.push_str(&line);
            if line == "\r\n" {
                break;
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        reader.get_mut().write_all(reply.as_bytes()).unwrap();
        head + &String::from_utf8(body).unwrap()
    });
    (base, handle)
}

/// A 200 response carrying `body`.
pub(crate) fn ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

/// A scratch workspace under the system temp directory, removed on drop.
pub(crate) struct Scratch {
    pub(crate) workspace: Workspace,
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Cleanup is best effort; a leftover temp directory does not change test results.
        match fs::remove_dir_all(&self.workspace.root) {
            Ok(()) | Err(_) => {}
        }
    }
}

impl Scratch {
    /// A fresh workspace whose `cargo`, `perf` and `time` are recording fakes that succeed.
    pub(crate) fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("aoc-ops-scratch-{}-{id}", std::process::id()));
        // A crash in an earlier run may have left this directory behind.
        match fs::remove_dir_all(&root) {
            Ok(()) | Err(_) => {}
        }
        fs::create_dir_all(root.join("crates/aoc-ops")).unwrap();
        fs::write(root.join("crates/aoc-ops/Cargo.toml"), "").unwrap();
        let tools = root.join(".tools");
        let scratch = Self {
            workspace: Workspace {
                base_url: String::new(),
                release: root.join("release"),
                cargo: tools.join("cargo").into(),
                perf: tools.join("perf").into(),
                time: tools.join("time").into(),
                root,
            },
        };
        // Fake cargo logs its arguments, one invocation per line.
        scratch.script(
            ".tools/cargo",
            "echo \"$*\" >> \"$(dirname \"$0\")/cargo.log\"",
        );
        // Fake perf reports a counter that grows with the input size, like real work.
        scratch.script(
            ".tools/perf",
            "shift 5; input=$(cat); n=$(printf %s \"$input\" | wc -c); \
             printf %s \"$input\" | \"$@\"; s=$?; echo \"$((1000 + n * 10)),,instructions:u,1,100.00,,\" >&2; exit $s",
        );
        // Fake GNU time reports fixed resources after the child's stderr.
        scratch.script(
            ".tools/time",
            "shift 3; \"$@\"; s=$?; echo 0.01,0.02,2048 >&2; exit $s",
        );
        scratch
    }

    pub(crate) fn root(&self) -> &Path {
        &self.workspace.root
    }

    pub(crate) fn path(&self, relative: &str) -> PathBuf {
        self.root().join(relative)
    }

    pub(crate) fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    pub(crate) fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap()
    }

    /// Writes an executable `/bin/sh` script.
    pub(crate) fn script(&self, relative: &str, body: &str) {
        self.write(relative, &format!("#!/bin/sh\n{body}\n"));
        fs::set_permissions(self.path(relative), fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Adds a solution module and its fake release binary, which runs `body` on stdin.
    pub(crate) fn solution(&self, year: &str, day: &str, module: &str, body: &str) {
        self.write(&format!("crates/aoc{year}/src/day{day}/{module}.rs"), "");
        let binary = if module == "primary" {
            format!("day{day}")
        } else {
            format!("day{day}-{module}")
        };
        self.script(&format!("release/{binary}"), body);
    }

    /// The fake cargo's recorded invocations.
    pub(crate) fn cargo_log(&self) -> String {
        fs::read_to_string(self.path(".tools/cargo.log")).unwrap_or_default()
    }
}
