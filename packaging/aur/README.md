# Publishing to the AUR

The `PKGBUILD` and `.SRCINFO` here are ready to publish. They were verified
locally with `makepkg -f` (build + tests + packaging).

## One-time setup

1. Create an account at https://aur.archlinux.org and add your SSH public key
   (Account → My Account → SSH Public Key).
2. Configure SSH for AUR in `~/.ssh/config`:

   ```
   Host aur.archlinux.org
     User aur
     IdentityFile ~/.ssh/id_ed25519
   ```

## Publish (first time and every update)

```bash
git clone ssh://aur@aur.archlinux.org/pomodomate-cli.git aur-pomodomate-cli
cd aur-pomodomate-cli
cp /path/to/repo/packaging/aur/PKGBUILD /path/to/repo/packaging/aur/.SRCINFO .
git add PKGBUILD .SRCINFO
git commit -m "Update to 0.2.0"
git push
```

After that, anyone on Arch can install with `yay -S pomodomate-cli`.

## Releasing a new version

1. Tag the new release on GitHub (the Release workflow builds the binaries).
2. Update `pkgver` in `PKGBUILD` and reset `pkgrel=1`.
3. Refresh the checksum: `updpkgsums` (or `sha256sum` of the new tag tarball).
4. Regenerate metadata: `makepkg --printsrcinfo > .SRCINFO`.
5. Commit and push both files to the AUR repo as above.
