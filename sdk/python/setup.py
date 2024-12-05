from setuptools import setup, find_packages

setup(
    name="rustray",
    version="0.1.0",
    packages=find_packages(),
    install_requires=[
        "grpcio>=1.51.1",
        "protobuf>=4.21.0",
        "numpy>=1.21.0",
        "typing-extensions>=4.0.0",
    ],
    author="RustRay Team",
    author_email="team@rustray.org",
    description="Python SDK for RustRay distributed computing framework",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    url="https://github.com/yourusername/rustray",
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: Apache Software License",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
    ],
    python_requires=">=3.7",
) 