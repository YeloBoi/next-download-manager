"""Runs only on a GitHub hosted runner; never installs into a user's profile."""
import pathlib
import shutil
import subprocess
import tempfile
import zipfile

from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.firefox.options import Options
from selenium.webdriver.support.ui import Select, WebDriverWait

with tempfile.TemporaryDirectory() as temporary:
    root = pathlib.Path(temporary)
    source = root / 'firefox'
    shutil.copytree('extensions/chromium', source)
    shutil.copyfile('extensions/firefox/manifest.json', source / 'manifest.json')
    subprocess.run(['npx', '--yes', 'web-ext@9', 'lint', '--source-dir', str(source), '--warnings-as-errors'], check=True)
    archive = root / 'ndm.zip'
    with zipfile.ZipFile(archive, 'w') as output:
        for path in source.rglob('*'):
            if path.is_file():
                output.write(path, path.relative_to(source))
    options = Options()
    options.binary_location = shutil.which('firefox')
    options.add_argument('-headless')
    options.add_argument('--allow-system-access')
    driver = webdriver.Firefox(options=options)
    try:
        addon = driver.install_addon(str(archive), temporary=True)
        assert addon == 'ndm@yeloboi.github.io', addon
        driver.set_context('chrome')
        popup = driver.execute_script("return WebExtensionPolicy.getByID(arguments[0]).getURL('popup.html')", addon)
        driver.set_context('content')
        driver.set_window_size(1000, 1000)
        driver.get(popup)
        wait = WebDriverWait(driver, 20)
        wait.until(lambda d: d.find_element(By.ID, 'connection').text)
        driver.find_element(By.ID, 'urls').send_keys('https://example.com/ndm-firefox-test.zip')
        driver.find_element(By.ID, 'collect').click()
        wait.until(lambda d: len(d.find_elements(By.CSS_SELECTOR, '#selection li')) == 1)
        driver.refresh()
        wait.until(lambda d: len(d.find_elements(By.CSS_SELECTOR, '#selection li')) == 1)
        driver.find_element(By.ID, 'queue').click()
        wait.until(lambda d: 'Links are saved' in d.find_element(By.ID, 'result').text)
        assert len(driver.find_elements(By.CSS_SELECTOR, '#selection li')) == 1
        Select(driver.find_element(By.ID, 'language')).select_by_value('fa')
        wait.until(lambda d: d.find_element(By.TAG_NAME, 'html').get_attribute('dir') == 'rtl')
        print('Firefox: temporary install, module background, saved selection, failed native send retention and Persian RTL passed.')
    finally:
        driver.quit()
