#!/bin/bash
set -e

# Find all pkgconfig folders for dev dependencies in /nix/store
PKG_CONFIG_PATHS=""
# Find matching dev directories
ALL_PATHS=$(for pkg in gtk4 libadwaita glib dav1d graphene gdk-pixbuf cairo pango harfbuzz epoxy pixman fribidi libthai xkbcommon wayland dbus vulkan jpeg libjpeg tiff libtiff zlib png libpng libdeflate lerc libwebp appstream; do
  for path in /nix/store/*${pkg}*-dev/lib/pkgconfig /nix/store/*${pkg}*-dev/share/pkgconfig /nix/store/*${pkg}*/lib/pkgconfig /nix/store/*${pkg}*/share/pkgconfig; do
    if [ -d "$path" ]; then
      echo "$path"
    fi
  done
done | sort -k1.44 -V -r | uniq)

for path in $ALL_PATHS; do
  if [[ ! "$PKG_CONFIG_PATHS" =~ "$path" ]]; then
    PKG_CONFIG_PATHS="${PKG_CONFIG_PATHS:+$PKG_CONFIG_PATHS:}$path"
  fi
done

export PKG_CONFIG_PATH="$PKG_CONFIG_PATHS"
export PKG_CONFIG_PATH_FOR_TARGET="$PKG_CONFIG_PATHS"
export LD_LIBRARY_PATH="/nix/store/dbz6pb9g67kpgpl95k8d85kzpxm1c32p-zlib-1.3.2/lib:$LD_LIBRARY_PATH"
exec "$@"
