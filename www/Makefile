all: file_format.html version_history.html index.html

file_format.html: ../docs/RDB_File_Format.md tmpl.html
	pandoc -S --toc -c style.css -o file_format.html --template tmpl.html ../docs/RDB_File_Format.md

version_history.html: ../docs/RDB_Version_History.md tmpl.html
	pandoc -S --toc -c style.css -o version_history.html --template tmpl.html ../docs/RDB_Version_History.md

index.html: index.md header.html footer.html
	 cat header.html > index.html
	 pandoc -t html index.md >> index.html
	 cat footer.html >> index.html

upload: version_history.html file_format.html index.html
	rsync -av file_format.html version_history.html index.html style.css rediger:/var/www/sites/rdb.fnordig.de/
