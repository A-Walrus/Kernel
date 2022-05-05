guestmount -a image.img -m /dev/sda1 Mountpoint
rsync -rvu --delete -L "FileSystem/" "Mountpoint"
guestunmount Mountpoint
cp image.img disk.img
