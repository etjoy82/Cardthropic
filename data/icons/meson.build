application_id = 'io.codeberg.emviolet.cardthropic'
legacy_icon_name = 'cardthropic'
icon_names = [application_id, legacy_icon_name]

scalable_dir = 'hicolor' / 'scalable' / 'apps'
foreach icon_name : icon_names
  install_data(
    scalable_dir / ('@0@.png').format(icon_name),
    install_dir: get_option('datadir') / 'icons' / scalable_dir
  )
endforeach

symbolic_dir = 'hicolor' / 'symbolic' / 'apps'
foreach icon_name : icon_names
  install_data(
    symbolic_dir / ('@0@-symbolic.png').format(icon_name),
    install_dir: get_option('datadir') / 'icons' / symbolic_dir
  )
endforeach

png_sizes = ['512x512', '256x256', '128x128', '64x64', '48x48', '32x32', '16x16']
foreach size : png_sizes
  png_dir = 'hicolor' / size / 'apps'
  foreach icon_name : icon_names
    install_data(
      png_dir / ('@0@.png').format(icon_name),
      install_dir: get_option('datadir') / 'icons' / png_dir
    )
  endforeach
endforeach

# Compatibility fallback for launchers that still consult /share/pixmaps.
foreach icon_name : icon_names
  install_data(
    'hicolor' / '128x128' / 'apps' / ('@0@.png').format(icon_name),
    install_dir: get_option('datadir') / 'pixmaps'
  )
endforeach
