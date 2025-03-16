include /usr/share/dpkg/default.mk

PACKAGE = net-sriov-tools
BUILDDIR ?= $(PACKAGE)-$(DEB_VERSION_UPSTREAM)

DEB=$(PACKAGE)_$(DEB_VERSION)_$(DEB_HOST_ARCH).deb
CURRDIR=${PWD}

CARGO ?= cargo
ifeq ($(BUILD_MODE), release)
CARGO_BUILD_ARGS += --release
CARGO_COMPILEDIR := target/release
else
CARGO_COMPILEDIR := target/debug
endif

PREFIX = /usr
BINDIR = debian/$(PACKAGE)/$(PREFIX)/sbin


COMPILED_BINS := \
	$(addprefix $(CARGO_COMPILEDIR)/,$(USR_BIN))


all:

$(BUILDDIR):
	rm -rf $@ $@.tmp; mkdir $@.tmp
	cp -a \
	  .cargo \
	  Cargo.toml \
	  Makefile \
	  src \
	  $@.tmp
	cp -a debian $@.tmp/
	mv $@.tmp $@


deb: $(DEB)
$(DEB): $(BUILDDIR)
	cd $(BUILDDIR); dpkg-buildpackage -b -us -uc
	lintian $(DEB)


.PHONY: install
install: cargo-build
	mkdir $(BINDIR) -p
	install -D -m 755 target/$(BUILD_MODE)/net-sriov-tools $(BINDIR)

.PHONY: cargo-build 
cargo-build: $(COMPILED_BINS)
	$(CARGO) build --$(BUILD_MODE)


.phony: clean
clean:
	rm -rf target build $(PACKAGE)-[0-9]* testdir
	rm -f $(PACKAGE)*.tar* *.deb packages packages.tmp *.build *.dsc *.buildinfo *.changes
	find . -name '*~' -exec rm {} ';'