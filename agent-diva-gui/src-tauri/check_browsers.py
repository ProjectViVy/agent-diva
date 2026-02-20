import os
import sys
import subprocess
import webbrowser

print("=== 浏览器检查 ===")
print("\n1. 尝试打开默认浏览器访问百度...")
try:
    webbrowser.open("https://www.baidu.com")
    print("已尝试打开浏览器")
except Exception as e:
    print(f"打开浏览器时出错: {e}")

print("\n2. 检查常见浏览器安装路径...")
browsers = {
    "Chrome": [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        os.path.expanduser(r"~\AppData\Local\Google\Chrome\Application\chrome.exe")
    ],
    "Edge": [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"
    ],
    "Firefox": [
        r"C:\Program Files\Mozilla Firefox\firefox.exe",
        r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe"
    ],
    "Opera": [
        r"C:\Program Files\Opera\launcher.exe",
        r"C:\Program Files (x86)\Opera\launcher.exe"
    ]
}

for browser_name, paths in browsers.items():
    found = False
    for path in paths:
        if os.path.exists(path):
            print(f"✓ {browser_name}: {path}")
            found = True
            break
    if not found:
        print(f"✗ {browser_name}: 未找到")

print("\n3. 检查环境变量中的浏览器...")
env_path = os.environ.get("PATH", "")
paths = env_path.split(";")
browser_exes = ["chrome.exe", "firefox.exe", "msedge.exe", "opera.exe", "iexplore.exe"]

for path in paths:
    if os.path.isdir(path):
        for exe in browser_exes:
            exe_path = os.path.join(path, exe)
            if os.path.exists(exe_path):
                print(f"✓ 在 PATH 中找到: {exe_path}")

print("\n4. 尝试使用 webbrowser 模块获取浏览器信息...")
try:
    browser = webbrowser.get()
    print(f"默认浏览器控制器: {browser}")
except Exception as e:
    print(f"获取浏览器信息时出错: {e}")