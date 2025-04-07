.PHONY:test

TARGET ?= archive
SOURCE ?= file1.txt file2.txt
FORMAT = zip
METHOD = zip
PASSWORD = mypassword

CA_ZIP = ./target/debug/ca-zip.exe

all: compress decompress

compress:
	$(CA_ZIP) $(TARGET) $(SOURCE)

decompress:
	$(CA_ZIP) -u $(TARGET).$(FORMAT)

clean:
	rm -f $(TARGET).$(FORMAT)
	rm -f $(SOURCE)

test:
	cargo build && ./target/debug/ca-zip --help