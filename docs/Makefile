all: file_format.html version_history.html

file_format.html: RDB_File_Format.md tmpl.html
	pandoc -S --toc -c style.css -o file_format.html --template tmpl.html RDB_File_Format.md

version_history.html: RDB_Version_History.md tmpl.html
	pandoc -S --toc -c style.css -o version_history.html --template tmpl.html RDB_Version_History.md

upload: version_history.html file_format.html
	rsync -av file_format.html version_history.html index.html style.css rediger:/var/www/sites/rdb.fnordig.de/
