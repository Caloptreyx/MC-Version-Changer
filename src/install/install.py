"""Version installer for the dev.caloptreyx.versionchanger Calagopus extension.

Runs inside the Wings installation container (server files mounted at /mnt/server).
The backend resolves the mcjars build and passes its validated installation steps in
`MVC_BUILD`; the Wings install progress / status files are used when available.

Flow: download every file of the build into a staging directory (nothing on the server
is touched until all downloads succeeded), then run the mcjars steps in order (move the
staged downloads into place, unzip, remove), rename `server.jar` to the egg's jar file,
put the Forge/NeoForge `unix_args.txt` at the server root and write `.mcvc-type.json`.
"""

import datetime
import http.client
import json
import os
import shutil
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path, PurePosixPath

ROOT = Path(os.environ.get("MVC_ROOT", "/mnt/server"))
TMP = ROOT / ".version-changer-tmp"
MARKER = ROOT / ".mcvc-type.json"
UNIX_ARGS = "unix_args.txt"

BUILD = os.environ.get("MVC_BUILD", "")
JARFILE = os.environ.get("MVC_JARFILE", "").strip() or "server.jar"
MODE = os.environ.get("MVC_MODE", "replace")
ACCEPT_EULA = os.environ.get("MVC_ACCEPT_EULA", "0") == "1"
USER_AGENT = os.environ.get("MVC_USER_AGENT", "Caloptreyx/MC-Version-Changer")
PROGRESS_FILE = os.environ.get("INSTALL_PROGRESS_FILE", "")
STATUS_FILE = os.environ.get("INSTALL_STATUS_FILE", "")

RETRYABLE_STATUS = {408, 425, 429, 500, 502, 503, 504}
ATTEMPTS = 6


class InstallError(Exception):
    pass


def log(message):
    print(f"[version-changer] {message}", flush=True)


def progress(percent, label):
    if not PROGRESS_FILE:
        return
    try:
        with open(PROGRESS_FILE, "w", encoding="utf-8") as handle:
            handle.write(f"{max(0, min(100, int(percent)))}/100 {label}\n")
    except OSError:
        pass


def report_failure(reason):
    if not STATUS_FILE:
        return
    try:
        with open(STATUS_FILE, "w", encoding="utf-8") as handle:
            handle.write(reason.replace("\n", " ")[:250] + "\n")
    except OSError:
        pass


def safe_relative(path):
    """Validates a relative path inside the server, returning a PurePosixPath or None."""
    if not path or "\\" in path or "\x00" in path or ":" in path:
        return None
    if any(part in ("", ".", "..") for part in path.split("/")):
        return None
    return PurePosixPath(path)


def require_relative(path, what):
    rel = safe_relative(path)
    if rel is None:
        raise InstallError(f"unsafe {what} path: {path!r}")
    return rel


def remove_path(path):
    if path.is_symlink() or path.is_file():
        path.unlink(missing_ok=True)
    elif path.is_dir():
        shutil.rmtree(path)


def human_size(size):
    for unit in ("B", "KiB", "MiB", "GiB"):
        if size < 1024 or unit == "GiB":
            return f"{size:.0f} {unit}" if unit == "B" else f"{size:.1f} {unit}"
        size /= 1024


def download(url, destination, expected_size, on_chunk):
    """Downloads `url` to `destination`, retrying transient failures."""
    host = urllib.parse.urlparse(url).hostname or url
    last_error = None
    for attempt in range(1, ATTEMPTS + 1):
        written = 0
        try:
            request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with urllib.request.urlopen(request, timeout=120) as response, open(destination, "wb") as out:
                while chunk := response.read(1024 * 1024):
                    out.write(chunk)
                    written += len(chunk)
                    on_chunk(len(chunk))
            if expected_size and written != expected_size:
                log(f"note: {host} sent {written} bytes, mcjars listed {expected_size}")
            return
        except urllib.error.HTTPError as err:
            last_error = err
            if err.code not in RETRYABLE_STATUS:
                raise InstallError(f"download from {host} failed with HTTP {err.code}") from err
        except (urllib.error.URLError, http.client.HTTPException, TimeoutError, OSError) as err:
            last_error = err
        on_chunk(-written)
        destination.unlink(missing_ok=True)
        if attempt == ATTEMPTS:
            break
        delay = 15 if isinstance(last_error, urllib.error.HTTPError) and last_error.code == 429 else 2**attempt
        log(f"download from {host} failed ({last_error}), retrying in {delay}s ({attempt}/{ATTEMPTS - 1})")
        time.sleep(delay)
    raise InstallError(f"download from {host} failed: {last_error}")


def stage_downloads(steps):
    """Downloads every `download` step into TMP. Returns {step index: staged path}."""
    downloads = [(index, step) for index, step in enumerate(steps) if step["action"] == "download"]
    total = sum(step.get("size") or 0 for _, step in downloads)
    done = 0
    last_reported = -1

    def on_chunk(count):
        nonlocal done, last_reported
        done += count
        if total:
            percent = 5 + 75 * min(done, total) / total
            if int(percent) != last_reported:
                last_reported = int(percent)
                progress(percent, f"Downloading ({human_size(done)} / {human_size(total)})")

    staged = {}
    for index, step in downloads:
        require_relative(step["file"], "download")
        size = step.get("size") or 0
        log(f"downloading {step['file']}" + (f" ({human_size(size)})" if size else ""))
        target = TMP / f"download-{index}"
        download(step["url"], target, size, on_chunk)
        staged[index] = target
    return staged


def extract(archive_path, location):
    """Extracts a zip into `location`, returning the relative paths of the extracted files."""
    extracted = []
    with zipfile.ZipFile(archive_path) as archive:
        members = archive.infolist()
        for info in members:
            require_relative(info.filename.rstrip("/"), "archive member")
        for info in members:
            member = PurePosixPath(info.filename.rstrip("/"))
            destination = location / member
            if info.is_dir():
                destination.mkdir(parents=True, exist_ok=True)
                continue
            destination.parent.mkdir(parents=True, exist_ok=True)
            if destination.is_symlink() or destination.is_dir():
                remove_path(destination)
            with archive.open(info) as source, open(destination, "wb") as out:
                shutil.copyfileobj(source, out, 1024 * 1024)
            extracted.append(destination.relative_to(ROOT).as_posix())
    return extracted


def apply_steps(steps, staged):
    """Runs the mcjars steps in order. Returns the relative paths of every file written."""
    written = []
    for index, step in enumerate(steps):
        action = step["action"]
        if action == "download":
            rel = require_relative(step["file"], "download")
            target = ROOT / rel
            target.parent.mkdir(parents=True, exist_ok=True)
            if target.exists() or target.is_symlink():
                remove_path(target)
            shutil.move(staged[index], target)
            written.append(rel.as_posix())
        elif action == "unzip":
            rel = require_relative(step["file"], "archive")
            location = step.get("location") or ""
            target = ROOT if location == "" else ROOT / require_relative(location, "unzip")
            log(f"extracting {rel}")
            written.extend(extract(ROOT / rel, target))
        elif action == "remove":
            rel = require_relative(step["location"], "remove")
            target = ROOT / rel
            if target.exists() or target.is_symlink():
                log(f"removing {rel}")
                remove_path(target)
        else:
            raise InstallError(f"unsupported installation step: {action}")
    return [path for path in dict.fromkeys(written) if (ROOT / path).exists()]


def place_jar(written):
    """Renames the produced `server.jar` to the jar file the egg starts."""
    if JARFILE == "server.jar" or "server.jar" not in written:
        return
    target = ROOT / require_relative(JARFILE, "jar file")
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists() or target.is_symlink():
        remove_path(target)
    (ROOT / "server.jar").rename(target)
    log(f"renamed server.jar to {JARFILE}")


def place_unix_args(written):
    """Forge/NeoForge eggs start with `@unix_args.txt` when it exists at the server root, so a
    stale copy from a previous install must go and the new build's copy is put in its place."""
    root_args = ROOT / UNIX_ARGS
    candidates = [path for path in written if path.startswith("libraries/") and path.endswith("/" + UNIX_ARGS)]
    if len(candidates) == 1:
        shutil.copyfile(ROOT / candidates[0], root_args)
        log(f"copied {candidates[0]} to {UNIX_ARGS}")
    elif root_args.is_file():
        root_args.unlink()
        log(f"removed the previous {UNIX_ARGS}")


def main():
    if not BUILD:
        raise InstallError("no build to install (MVC_BUILD is empty)")
    build = json.loads(BUILD)
    steps = [step for group in build.get("installation") or [] for step in group]
    if not any(step.get("action") == "download" for step in steps):
        raise InstallError("the build has no files to install")
    require_relative(JARFILE, "jar file")

    name = build.get("typeName") or build["type"]
    log(
        f"installing {name} {build.get('version') or ''} (build {build.get('buildName')})"
        + (" after a clean install" if MODE == "wipe" else ", keeping worlds and settings")
    )

    ROOT.mkdir(parents=True, exist_ok=True)
    shutil.rmtree(TMP, ignore_errors=True)
    TMP.mkdir(parents=True)

    progress(5, "Downloading")
    staged = stage_downloads(steps)

    progress(82, "Installing")
    written = apply_steps(steps, staged)
    place_jar(written)
    place_unix_args(written)

    if ACCEPT_EULA:
        (ROOT / "eula.txt").write_text("eula=true\n", encoding="utf-8")

    marker = {
        "type": build["type"],
        "version": build.get("version"),
        "minecraftVersion": build.get("minecraftVersion"),
        "projectVersion": build.get("projectVersion"),
        "buildId": build.get("buildId"),
        "buildName": build.get("buildName"),
        "installedAt": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "installer": "dev.caloptreyx.versionchanger",
    }
    MARKER.write_text(json.dumps(marker, indent=2) + "\n", encoding="utf-8")

    shutil.rmtree(TMP, ignore_errors=True)
    progress(100, "Done")
    log(f"installed {name} {build.get('version') or ''} (build {build.get('buildName')}) as {JARFILE}")


if __name__ == "__main__":
    try:
        main()
    except InstallError as err:
        log(f"installation failed: {err}")
        report_failure(str(err))
        shutil.rmtree(TMP, ignore_errors=True)
        sys.exit(1)
    except Exception as err:  # noqa: BLE001 - surface anything unexpected in the console
        log(f"installation failed unexpectedly: {err!r}")
        report_failure(f"unexpected error: {err}")
        shutil.rmtree(TMP, ignore_errors=True)
        sys.exit(1)
