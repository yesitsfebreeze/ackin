"""Run the ASP browser against a project's existing host."""
import argparse
import asyncio
import json
import os

from .client import Client
from .model import Context


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dir", default=os.getcwd())
    parser.add_argument("--once", action="store_true", help="Print one ASP snapshot as JSON without opening a terminal")
    args = parser.parse_args()
    if args.once:
        async def snapshot():
            client, model = Client(os.path.abspath(args.dir)), Context()
            model.types = await client.asp("types")
            model.merge(await client.asp("expand", entity="asp:root", depth=1, limit=256))
            model.activity(await client.asp("activity"))
            print(json.dumps({"items": model.items(), "observations": model.records, "errors": model.errors}, ensure_ascii=False))
        try:
            asyncio.run(snapshot())
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            parser.exit(1, f"Scope: {error or 'Host request timed out'}\n")
    else:
        from .app import Scope
        Scope(os.path.abspath(args.dir)).run()


if __name__ == "__main__":
    main()
