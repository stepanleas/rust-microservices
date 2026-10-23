#!/usr/bin/env bash
# Prepares a stopped lab VM before `vagrant up` (a Vagrantfile trigger), for two things
# vagrant-libvirt 0.12 can't do:
# - a new memory size: it calls virDomainSetMemory, which fails on a stopped VM
#   ("domain is not running"), so the size is written to the VM's definition here;
# - free page reporting on the virtio balloon, so the VM hands its free memory back to the host.
# A VM created by that same `up` gets both from its next start.
# Usage: vm-before-up.sh <libvirt domain> <memory in MiB>
set -euo pipefail

domain=$1
memory_kib=$(($2 * 1024))
virsh=(virsh --connect qemu:///system)

"${virsh[@]}" dominfo "$domain" &>/dev/null || exit 0
[[ $("${virsh[@]}" domstate "$domain") == "shut off" ]] || exit 0

xml=$("${virsh[@]}" dumpxml --inactive "$domain")
if ! grep -q "<memory unit='KiB'>$memory_kib</memory>" <<<"$xml"; then
  "${virsh[@]}" setmaxmem "$domain" "$memory_kib" --config
  "${virsh[@]}" setmem "$domain" "$memory_kib" --config
  echo "$domain: memory $2 MiB"
  xml=$("${virsh[@]}" dumpxml --inactive "$domain")
fi
if ! grep -q "freePageReporting='on'" <<<"$xml"; then
  sed "s|<memballoon model='virtio'>|<memballoon model='virtio' freePageReporting='on'>|" <<<"$xml" \
    | "${virsh[@]}" define /dev/stdin >/dev/null
  echo "$domain: free page reporting on"
fi
