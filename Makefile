PREFIX ?= /usr/local
BINDIR ?= $(DESTDIR)$(PREFIX)/bin
SYSTEMD_DIR ?= $(DESTDIR)/usr/lib/systemd/user

.PHONY: install install_systemd clean

target/release/track:
	cargo build --release

install: target/release/track
	install -d $(BINDIR)
	install -m 0755 target/release/track $(BINDIR)/track

install_systemd: install
	install -d $(SYSTEMD_DIR)
	install -m 0644 track-notify.service $(SYSTEMD_DIR)/track-notify.service
	systemctl --user daemon-reload || true

clean:
	cargo clean
