from setuptools import setup
from glob import glob

setup(
    name='sailtrack-core',
    version='0.0.1',
    packages=['sailtrackd'],
    url='https://github.com/metis-vela-unipd/sailtrack-core',
    license='GPL-3.0',
    author='Métis Vela Unipd',
    author_email='matteo.carnelos@studenti.unipd.it',
    description='Core package of the SailTrack system.',
    install_requires=[

    ],
    data_files=[
        ('/etc/systemd/system', glob('systemd/*'))
    ]
)
