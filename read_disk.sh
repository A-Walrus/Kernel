sudo losetup -D
sudo losetup -f -P disk.img
sudo mount /dev/loop0p1 Mountpoint
sudo rsync -rvu --delete -L "Mountpoint/" "DiskFS"
sudo umount /dev/loop0p1
sudo losetup -D
