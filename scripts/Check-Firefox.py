"""Runs only on a GitHub hosted runner; never installs into a user's profile."""
import pathlib
import json
import os
import signal
import shutil
import subprocess
import tempfile
import zipfile
import sys
import time

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.firefox.options import Options
from selenium.webdriver.support.ui import Select, WebDriverWait

with tempfile.TemporaryDirectory() as temporary:
    root = pathlib.Path(temporary)
    source = root / 'firefox'
    shutil.copytree('extensions/chromium', source)
    shutil.copyfile('extensions/firefox/manifest.json', source / 'manifest.json')
    # Mozilla also warns about Android minimum versions; NDM targets Windows desktop.
    subprocess.run(['npx', '--yes', 'web-ext@9', 'lint', '--source-dir', str(source)], check=True)
    archive = root / 'ndm.zip'
    with zipfile.ZipFile(archive, 'w') as output:
        for path in source.rglob('*'):
            if path.is_file():
                output.write(path, path.relative_to(source))
    options = Options()
    options.binary_location = shutil.which('firefox')
    options.add_argument('-headless')
    options.add_argument('--allow-system-access')
    # This fixture owns only a hosted runner's fake desktop host, never a user profile.
    assert os.environ.get('GITHUB_ACTIONS') == 'true'
    online = root / 'desktop-online'
    accept_add = root / 'accept-add'
    native_log = root / 'native-messages.jsonl'
    host = root / 'ndm-fixture.py'
    host.write_text(f'#!{sys.executable}\nONLINE = {str(online)!r}\nACCEPT_ADD = {str(accept_add)!r}\nLOG = {str(native_log)!r}\n' + '''
import json, os, pathlib, struct, sys
while True:
    header = sys.stdin.buffer.read(4)
    if not header: break
    size = struct.unpack('<I', header)[0]
    if size > 256 * 1024: break
    message = json.loads(sys.stdin.buffer.read(size))
    accepted = pathlib.Path(ONLINE).exists() and (message['kind'] == 'ping' or pathlib.Path(ACCEPT_ADD).exists())
    with open(LOG, 'a') as log:
        log.write(json.dumps({'accepted': accepted, 'kind': message['kind'], 'start': message['start'], 'urls': message['urls'], 'pid': os.getpid()}) + '\\n')
    response = json.dumps({'version': 1, 'request_id': message['request_id'], 'accepted': accepted}).encode()
    sys.stdout.buffer.write(struct.pack('<I', len(response)) + response)
    sys.stdout.buffer.flush()
''', encoding='utf8')
    host.chmod(0o755)
    manifests = pathlib.Path.home() / '.mozilla' / 'native-messaging-hosts'
    manifests.mkdir(parents=True, exist_ok=True)
    native_manifest = manifests / 'local.ndm.bridge.json'
    assert not native_manifest.exists(), 'Fixture must not replace another native host'
    native_manifest.write_text(json.dumps({'name': 'local.ndm.bridge', 'description': 'NDM CI fixture',
        'path': str(host), 'type': 'stdio', 'allowed_extensions': ['ndm@yeloboi.github.io']}), encoding='utf8')
    driver = None
    try:
        driver = webdriver.Firefox(options=options)
        addon = driver.install_addon(str(archive), temporary=True)
        assert addon == 'ndm@yeloboi.github.io', addon
        wait = WebDriverWait(driver, 25)
        def messages():
            return [json.loads(line) for line in native_log.read_text().splitlines()] if native_log.exists() else []
        wait.until(lambda _: any(not row['accepted'] for row in messages()))
        before = len(messages())
        online.touch()
        wait.until(lambda _: any(row['accepted'] for row in messages()[before:]))
        background_pid = messages()[-1]['pid']
        driver.set_context('chrome')
        popup = driver.execute_script("return WebExtensionPolicy.getByID(arguments[0]).getURL('popup.html')", addon)
        driver.set_context('content')
        driver.set_window_size(1000, 1000)
        driver.get(popup)
        wait.until(lambda d: d.find_element(By.ID, 'connection').text == 'Desktop connected')
        driver.find_element(By.ID, 'urls').send_keys('https://example.com/ndm-firefox-test.zip')
        driver.find_element(By.ID, 'collect').click()
        wait.until(lambda d: len(d.find_elements(By.CSS_SELECTOR, '#selection li')) == 1)
        driver.refresh()
        wait.until(lambda d: len(d.find_elements(By.CSS_SELECTOR, '#selection li')) == 1)
        driver.find_element(By.ID, 'queue').click()
        wait.until(lambda d: 'Links are saved' in d.find_element(By.ID, 'result').text)
        assert len(driver.find_elements(By.CSS_SELECTOR, '#selection li')) == 1
        driver.get('about:blank')  # Unload the popup completely.
        time.sleep(35)  # Cross the event-page idle period with no popup open.
        before = len(messages()); online.unlink()
        wait.until(lambda _: any(not row['accepted'] for row in messages()[before:]))
        before = len(messages()); online.touch()
        wait.until(lambda _: any(row['accepted'] for row in messages()[before:]))
        assert messages()[-1]['pid'] == background_pid, 'Closing the popup must preserve the background native port'
        os.kill(background_pid, signal.SIGTERM)
        wait.until(lambda _: any(row['accepted'] and row['pid'] != background_pid for row in messages()))
        driver.get(popup)
        wait.until(lambda d: d.find_element(By.ID, 'connection').text == 'Desktop connected')
        accept_add.touch()
        driver.find_element(By.ID, 'now').click()
        wait.until(lambda d: 'Accepted by NDM' in d.find_element(By.ID, 'result').text)
        assert not driver.find_elements(By.CSS_SELECTOR, '#selection li')
        sent = [row for row in messages() if row['kind'] == 'add' and row['accepted']]
        assert sent[-1]['start'] is True
        assert sent[-1]['urls'] == ['https://example.com/ndm-firefox-test.zip']
        Select(driver.find_element(By.ID, 'language')).select_by_value('fa')
        wait.until(lambda d: d.find_element(By.TAG_NAME, 'html').get_attribute('dir') == 'rtl')
        print('Firefox passed: automatic connection without popup; background recovery; failed send retention; Download all sends exact URLs with start=true and clears only acknowledged selection; Persian RTL.')
    finally:
        if driver: driver.quit()
        native_manifest.unlink(missing_ok=True)
