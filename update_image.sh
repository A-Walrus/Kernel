sudo losetup -D
sudo losetup -f -P image.img
sudo mount /dev/loop0p1 Mountpoint
sudo rsync -rvu --delete -L "FileSystem/" "Mountpoint"
sudo umount /dev/loop0p1
sudo losetup -D
