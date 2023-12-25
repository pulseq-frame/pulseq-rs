import venv
import subprocess
from os import path, mkdir


def shell(*args):
    if len(args) == 1:
        args = args[0].split()
    subprocess.Popen(list(args)).wait()


# We test the parser on the example sequences provided by PyPulseq.

# Versions available on PyPI:
# 1.0.0, 1.0.0.post1,
# 1.2.0.post1, 1.2.0.post2, 1.2.0.post3, 1.2.0.post4,
# 1.3.1, 1.3.1.post1,
# 1.4.0

# Only the newest verions include their example scripts. The only backwards-
# incompatible changes in the file format were between 1.3 and 1.4, so testing
# those two versions should be sufficient.
versions = ["1.3.1.post1", "1.4.0"]

cwd = path.dirname(__file__)
assets_dir = path.join(cwd, "..", "assets")
venv_dir = path.join(cwd, "venv")
venv_python = path.join(cwd, "venv", "scripts", "python.exe")
venv_pip = path.join(cwd, "venv", "scripts", "pip.exe")

venv.main([venv_dir, "--upgrade-deps"])
# Only install dependencies once, from the newest version
shell(venv_pip, "install", f"pypulseq=={versions[-1]}")


for version in versions:
    try:
        mkdir(path.join(assets_dir, version))
    except FileExistsError:
        pass
    shell(
        venv_pip, "install", f"pypulseq=={version}",
        "--force-reinstall", "--no-deps"
    )
    shell(venv_python, path.join(cwd, version + ".py"))
