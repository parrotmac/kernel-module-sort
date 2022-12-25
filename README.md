A module that can determine the proper loading order for Linux kernel modules based on a built Kernel tree.

## Example

```bash
# Download a built kernel archive (this will probably 404 -- replace with something current)
curl -fsSL -o kernel.tar.zst https://mirrors.xmission.com/archlinux/core/os/x86_64/linux-6.1.1.arch1-1-x86_64.pkg.tar.zst

# Extract downloaded archive
tar -xf kernel.tar.zst

# Decompress all kernel modules in-place (being able to inspect compressed modules would be a great new feature)
find ./usr/lib/ -type f -name '*.ko.zst' | xargs unzstd

# Inspect kernel and modules to determine proper loading order for specified 'target' module
cargo run -- --kernel ./usr/lib/modules/*/vmlinuz --modules ./usr/lib/modules/*/kernel/ --target wireguard.ko
Parsing kernel...done.
Parsing modules...done.
Resolving dependency tree...done.
usr/lib/modules/6.1.1-arch1-1/kernel/lib/crypto/libcurve25519-generic.ko
usr/lib/modules/6.1.1-arch1-1/kernel/arch/x86/crypto/curve25519-x86_64.ko
usr/lib/modules/6.1.1-arch1-1/kernel/lib/crypto/libchacha.ko
usr/lib/modules/6.1.1-arch1-1/kernel/arch/x86/crypto/chacha-x86_64.ko
usr/lib/modules/6.1.1-arch1-1/kernel/arch/x86/crypto/poly1305-x86_64.ko
usr/lib/modules/6.1.1-arch1-1/kernel/lib/crypto/libchacha20poly1305.ko
usr/lib/modules/6.1.1-arch1-1/kernel/net/ipv4/udp_tunnel.ko
usr/lib/modules/6.1.1-arch1-1/kernel/net/ipv6/ip6_udp_tunnel.ko
usr/lib/modules/6.1.1-arch1-1/kernel/drivers/net/wireguard/wireguard.ko
```
