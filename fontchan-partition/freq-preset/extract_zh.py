import urllib
import hashlib
import os.path as osp
import urllib.request

pwd = osp.dirname(osp.abspath(__file__))
raw_html = osp.join(pwd, "raw_zh.html")
raw_html_sha256 = "350d38a9ab5ba52da30d4ed867fe14f7e7eb0644554fb4bad3efc5e5b03dedc9"


def calculate_sha256(file_path):
    if not osp.exists(file_path):
        return None
    sha256 = hashlib.sha256()
    with open(file_path, "rb") as f:
        for block in iter(lambda: f.read(4096), b""):
            sha256.update(block)
    return sha256.hexdigest()


def download_with_progress(url, output_path):
    def reporthook(block_num, block_size, total_size):
        downloaded = block_num * block_size
        if total_size > 0:
            progress = downloaded / total_size * 100
            print(f"\rDownloading: {progress:.2f}%", end="")
        else:
            print(f"\rDownloaded {downloaded} bytes", end="")

    urllib.request.urlretrieve(url, output_path, reporthook)
    print("\nDownload complete.")


url = "https://lingua.mtsu.edu/chinese-computing/statistics/char/list.php?Which=TO"
if calculate_sha256(raw_html) != raw_html_sha256:
    download_with_progress(url, raw_html)
print(calculate_sha256(raw_html))
if calculate_sha256(raw_html) != raw_html_sha256:
    raise RuntimeError("Downloaded file is corrupted.")


with open(raw_html, "r", encoding="gb18030") as f:
    html = f.read()

import re

freq_table = re.findall(r"<pre>(.+)</pre>", html)[0]
entries = re.findall(r"(<br>|^)\s*(\d+)\s+(.)", freq_table)
with open(osp.join(pwd, "freq_zh.rs"), "w", encoding="utf-8") as f:
    codes = list(range(257)) + [ord(c) for _, _, c in entries]
    f.write("const FREQ_PRESET_ZH: &'static [char] = &[")
    f.write(",\n".join(map(lambda c: f"'\\u{{{c:X}}}'", codes)))
    f.write("];")
