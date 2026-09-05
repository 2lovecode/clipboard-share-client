# clipboard-share-client — common developer targets
# Requires: Node.js, Rust (stable), make (Git Bash / WSL / MSYS2)

.PHONY: help install dev build check test frontend-build clean

help: ## Show available targets
	@echo "Targets:"
	@echo "  make install         npm install"
	@echo "  make dev             npm run tauri dev"
	@echo "  make build           npm run tauri build"
	@echo "  make check           cargo check (src-tauri)"
	@echo "  make test            cargo test (src-tauri)"
	@echo "  make frontend-build  npm run build (Vue/Vite only)"
	@echo "  make clean           remove node_modules and Rust target"

install: ## Install frontend dependencies
	npm install

dev: ## Run Tauri + Vue in development
	npm run tauri dev

build: ## Production Tauri package
	npm run tauri build

check: ## Type-check / compile Rust backend
	cd src-tauri && cargo check

test: ## Run Rust tests
	cd src-tauri && cargo test

frontend-build: ## Build frontend only (vue-tsc + vite)
	npm run build

clean: ## Remove build artifacts and node_modules
	rm -rf node_modules ui/dist
	cd src-tauri && cargo clean
