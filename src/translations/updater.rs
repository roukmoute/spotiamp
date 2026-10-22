//! Updater messages and trusted diagnostic prefixes. Values: German, Spanish, Italian, Portuguese.

pub(super) const ENTRIES: &[(&str, [&str; 4])] = &[
    (
        "A file in the archive has an incorrect size.",
        [
            "Eine Datei im Archiv hat eine falsche Größe.",
            "Un archivo del paquete tiene un tamaño incorrecto.",
            "Un file nell'archivio ha una dimensione errata.",
            "Um ficheiro no arquivo tem um tamanho incorreto.",
        ],
    ),
    (
        "An update is available, but its SHA-256 checksum is missing or invalid",
        [
            "Ein Update ist verfügbar, aber die SHA-256-Prüfsumme fehlt oder ist ungültig",
            "Hay una actualización disponible, pero su suma de comprobación SHA-256 falta o no es válida",
            "È disponibile un aggiornamento, ma il codice di controllo SHA-256 manca o non è valido",
            "Há uma atualização disponível, mas a soma de verificação SHA-256 está em falta ou é inválida",
        ],
    ),
    (
        "An update is available, but its URL is invalid",
        [
            "Ein Update ist verfügbar, aber die URL ist ungültig",
            "Hay una actualización disponible, pero su URL no es válida",
            "È disponibile un aggiornamento, ma il suo URL non è valido",
            "Há uma atualização disponível, mas o URL é inválido",
        ],
    ),
    (
        "An update is available, but its size is invalid",
        [
            "Ein Update ist verfügbar, aber die Größenangabe ist ungültig",
            "Hay una actualización disponible, pero su tamaño no es válido",
            "È disponibile un aggiornamento, ma la sua dimensione non è valida",
            "Há uma atualização disponível, mas o tamanho é inválido",
        ],
    ),
    (
        "Another update is already running in this directory.",
        [
            "In diesem Ordner läuft bereits ein anderes Update.",
            "Ya hay otra actualización en curso en esta carpeta.",
            "Un altro aggiornamento è già in corso in questa cartella.",
            "Já está outra atualização em curso nesta pasta.",
        ],
    ),
    (
        "Backups retained in",
        [
            "Sicherungskopien aufbewahrt in",
            "Copias de seguridad conservadas en",
            "Copie di sicurezza conservate in",
            "Cópias de segurança guardadas em",
        ],
    ),
    (
        "Confirmation already exists.",
        [
            "Die Bestätigung ist bereits vorhanden.",
            "La confirmación ya existe.",
            "La conferma esiste già.",
            "A confirmação já existe.",
        ],
    ),
    (
        "Conflicting destinations.",
        [
            "Widersprüchliche Zielpfade.",
            "Destinos en conflicto.",
            "Destinazioni in conflitto.",
            "Destinos em conflito.",
        ],
    ),
    (
        "Could not back up the file",
        [
            "Die Datei konnte nicht gesichert werden",
            "No se pudo crear una copia de seguridad del archivo",
            "Impossibile creare una copia di sicurezza del file",
            "Não foi possível criar uma cópia de segurança do ficheiro",
        ],
    ),
    (
        "Could not check releases",
        [
            "Die Versionssuche ist fehlgeschlagen",
            "No se pudieron buscar las versiones",
            "Impossibile cercare le versioni",
            "Não foi possível procurar as versões",
        ],
    ),
    (
        "Could not clean up the update",
        [
            "Die Updatedateien konnten nicht bereinigt werden",
            "No se pudieron eliminar los archivos temporales de actualización",
            "Impossibile rimuovere i file temporanei dell'aggiornamento",
            "Não foi possível remover os ficheiros temporários da atualização",
        ],
    ),
    (
        "Could not copy the helper",
        [
            "Das Hilfsprogramm konnte nicht kopiert werden",
            "No se pudo copiar el asistente",
            "Impossibile copiare l'assistente",
            "Não foi possível copiar o assistente",
        ],
    ),
    (
        "Could not create backups",
        [
            "Sicherungskopien konnten nicht erstellt werden",
            "No se pudieron crear las copias de seguridad",
            "Impossibile creare le copie di sicurezza",
            "Não foi possível criar as cópias de segurança",
        ],
    ),
    (
        "Could not create the directory",
        [
            "Der Ordner konnte nicht erstellt werden",
            "No se pudo crear la carpeta",
            "Impossibile creare la cartella",
            "Não foi possível criar a pasta",
        ],
    ),
    (
        "Could not create the update file",
        [
            "Die Updatedatei konnte nicht erstellt werden",
            "No se pudo crear el archivo de actualización",
            "Impossibile creare il file di aggiornamento",
            "Não foi possível criar o ficheiro de atualização",
        ],
    ),
    (
        "Could not download the update",
        [
            "Das Update konnte nicht heruntergeladen werden",
            "No se pudo descargar la actualización",
            "Impossibile scaricare l'aggiornamento",
            "Não foi possível transferir a atualização",
        ],
    ),
    (
        "Could not find the executable",
        [
            "Die ausführbare Datei wurde nicht gefunden",
            "No se encontró el ejecutable",
            "Impossibile trovare l'eseguibile",
            "Não foi possível encontrar o executável",
        ],
    ),
    (
        "Could not find the player filename.",
        [
            "Der Dateiname des Players wurde nicht gefunden.",
            "No se encontró el nombre del archivo del reproductor.",
            "Impossibile trovare il nome del file del lettore.",
            "Não foi possível encontrar o nome do ficheiro do leitor.",
        ],
    ),
    (
        "Could not finish the download.",
        [
            "Der Download konnte nicht abgeschlossen werden.",
            "No se pudo completar la descarga.",
            "Impossibile completare il download.",
            "Não foi possível concluir a transferência.",
        ],
    ),
    (
        "Could not inspect the process",
        [
            "Der Prozessstatus konnte nicht ermittelt werden",
            "No se pudo consultar el estado del proceso",
            "Impossibile verificare lo stato del processo",
            "Não foi possível consultar o estado do processo",
        ],
    ),
    (
        "Could not locate the player directory.",
        [
            "Der Ordner des Players wurde nicht gefunden.",
            "No se encontró la carpeta del reproductor.",
            "Impossibile trovare la cartella del lettore.",
            "Não foi possível encontrar a pasta do leitor.",
        ],
    ),
    (
        "Could not locate the player.",
        [
            "Der Player wurde nicht gefunden.",
            "No se encontró el reproductor.",
            "Impossibile trovare il lettore.",
            "Não foi possível encontrar o leitor.",
        ],
    ),
    (
        "Could not open the process",
        [
            "Auf den Prozess konnte nicht zugegriffen werden",
            "No se pudo acceder al proceso",
            "Impossibile accedere al processo",
            "Não foi possível aceder ao processo",
        ],
    ),
    (
        "Could not prepare confirmation",
        [
            "Die Bestätigung konnte nicht vorbereitet werden",
            "No se pudo preparar la confirmación",
            "Impossibile preparare la conferma",
            "Não foi possível preparar a confirmação",
        ],
    ),
    (
        "Could not prepare the directory.",
        [
            "Der Ordner konnte nicht vorbereitet werden.",
            "No se pudo preparar la carpeta.",
            "Impossibile preparare la cartella.",
            "Não foi possível preparar a pasta.",
        ],
    ),
    (
        "Could not prepare the update.",
        [
            "Das Update konnte nicht vorbereitet werden.",
            "No se pudo preparar la actualización.",
            "Impossibile preparare l'aggiornamento.",
            "Não foi possível preparar a atualização.",
        ],
    ),
    (
        "Could not read releases",
        [
            "Die Versionsliste konnte nicht gelesen werden",
            "No se pudo leer la lista de versiones",
            "Impossibile leggere l'elenco delle versioni",
            "Não foi possível ler a lista de versões",
        ],
    ),
    (
        "Could not remove the new file",
        [
            "Die neue Datei konnte nicht entfernt werden",
            "No se pudo eliminar el archivo nuevo",
            "Impossibile rimuovere il nuovo file",
            "Não foi possível remover o novo ficheiro",
        ],
    ),
    (
        "Could not replace the file",
        [
            "Die Datei konnte nicht ersetzt werden",
            "No se pudo sustituir el archivo",
            "Impossibile sostituire il file",
            "Não foi possível substituir o ficheiro",
        ],
    ),
    (
        "Could not restart the player",
        [
            "Der Player konnte nicht neu gestartet werden",
            "No se pudo reiniciar el reproductor",
            "Impossibile riavviare il lettore",
            "Não foi possível reiniciar o leitor",
        ],
    ),
    (
        "Could not restart:",
        [
            "Neustart fehlgeschlagen:",
            "No se pudo reiniciar:",
            "Impossibile riavviare:",
            "Não foi possível reiniciar:",
        ],
    ),
    (
        "Could not restore the file",
        [
            "Die Datei konnte nicht wiederhergestellt werden",
            "No se pudo restaurar el archivo",
            "Impossibile ripristinare il file",
            "Não foi possível restaurar o ficheiro",
        ],
    ),
    (
        "Could not save confirmation",
        [
            "Die Bestätigung konnte nicht gespeichert werden",
            "No se pudo guardar la confirmación",
            "Impossibile salvare la conferma",
            "Não foi possível guardar a confirmação",
        ],
    ),
    (
        "Could not save the update",
        [
            "Das Update konnte nicht gespeichert werden",
            "No se pudo guardar la actualización",
            "Impossibile salvare l'aggiornamento",
            "Não foi possível guardar a atualização",
        ],
    ),
    (
        "Could not save the update.",
        [
            "Das Update konnte nicht gespeichert werden.",
            "No se pudo guardar la actualización.",
            "Impossibile salvare l'aggiornamento.",
            "Não foi possível guardar a atualização.",
        ],
    ),
    (
        "Could not set update file permissions.",
        [
            "Die Zugriffsrechte der Updatedateien konnten nicht gesetzt werden.",
            "No se pudieron establecer los permisos de los archivos de actualización.",
            "Impossibile impostare i permessi dei file di aggiornamento.",
            "Não foi possível definir as permissões dos ficheiros de atualização.",
        ],
    ),
    (
        "Could not start the helper",
        [
            "Das Hilfsprogramm konnte nicht gestartet werden",
            "No se pudo iniciar el asistente",
            "Impossibile avviare l'assistente",
            "Não foi possível iniciar o assistente",
        ],
    ),
    (
        "Could not start the update.",
        [
            "Das Update konnte nicht gestartet werden.",
            "No se pudo iniciar la actualización.",
            "Impossibile avviare l'aggiornamento.",
            "Não foi possível iniciar a atualização.",
        ],
    ),
    (
        "Could not validate confirmation",
        [
            "Die Bestätigung konnte nicht validiert werden",
            "No se pudo validar la confirmación",
            "Impossibile convalidare la conferma",
            "Não foi possível validar a confirmação",
        ],
    ),
    (
        "Could not write confirmation",
        [
            "Die Bestätigung konnte nicht geschrieben werden",
            "No se pudo escribir la confirmación",
            "Impossibile scrivere la conferma",
            "Não foi possível escrever a confirmação",
        ],
    ),
    (
        "Could not write the update",
        [
            "Das Update konnte nicht geschrieben werden",
            "No se pudo escribir la actualización",
            "Impossibile scrivere l'aggiornamento",
            "Não foi possível escrever a atualização",
        ],
    ),
    (
        "Could not write the update.",
        [
            "Das Update konnte nicht geschrieben werden.",
            "No se pudo escribir la actualización.",
            "Impossibile scrivere l'aggiornamento.",
            "Não foi possível escrever a atualização.",
        ],
    ),
    (
        "Current version not recognized",
        [
            "Die aktuelle Version wurde nicht erkannt",
            "No se reconoce la versión actual",
            "Versione attuale non riconosciuta",
            "A versão atual não foi reconhecida",
        ],
    ),
    (
        "Download interrupted",
        [
            "Download unterbrochen",
            "Descarga interrumpida",
            "Download interrotto",
            "Transferência interrompida",
        ],
    ),
    (
        "Duplicate or invalid destination.",
        [
            "Doppelter oder ungültiger Zielpfad.",
            "Destino duplicado o no válido.",
            "Destinazione duplicata o non valida.",
            "Destino duplicado ou inválido.",
        ],
    ),
    (
        "Executable verification failed.",
        [
            "Die Prüfung der ausführbaren Datei ist fehlgeschlagen.",
            "La verificación del ejecutable ha fallado.",
            "Verifica dell'eseguibile non riuscita.",
            "A verificação do executável falhou.",
        ],
    ),
    (
        "Expected a regular directory.",
        [
            "Ein regulärer Ordner wurde erwartet.",
            "Se esperaba una carpeta normal.",
            "Era prevista una cartella normale.",
            "Era esperada uma pasta normal.",
        ],
    ),
    (
        "Expected a regular file.",
        [
            "Eine reguläre Datei wurde erwartet.",
            "Se esperaba un archivo normal.",
            "Era previsto un file normale.",
            "Era esperado um ficheiro normal.",
        ],
    ),
    (
        "Expired or invalid update manifest.",
        [
            "Abgelaufenes oder ungültiges Updatemanifest.",
            "Manifiesto de actualización caducado o no válido.",
            "Manifesto di aggiornamento scaduto o non valido.",
            "Manifesto de atualização expirado ou inválido.",
        ],
    ),
    (
        "GitHub connection unavailable",
        [
            "Die Verbindung zu GitHub ist nicht verfügbar",
            "La conexión con GitHub no está disponible",
            "Connessione a GitHub non disponibile",
            "A ligação ao GitHub não está disponível",
        ],
    ),
    (
        "GitHub returned HTTP",
        [
            "GitHub antwortete mit HTTP",
            "GitHub respondió con HTTP",
            "GitHub ha risposto con HTTP",
            "O GitHub respondeu com HTTP",
        ],
    ),
    (
        "Incompatible package version or architecture.",
        [
            "Inkompatible Paketversion oder Architektur.",
            "Versión o arquitectura del paquete incompatible.",
            "Versione o architettura del pacchetto incompatibile.",
            "Versão ou arquitetura do pacote incompatível.",
        ],
    ),
    (
        "Invalid directory.",
        [
            "Ungültiger Ordner.",
            "Carpeta no válida.",
            "Cartella non valida.",
            "Pasta inválida.",
        ],
    ),
    (
        "Invalid download metadata",
        [
            "Ungültige Download-Metadaten",
            "Metadatos de descarga no válidos",
            "Metadati del download non validi",
            "Metadados de transferência inválidos",
        ],
    ),
    (
        "Invalid file list.",
        [
            "Ungültige Dateiliste.",
            "Lista de archivos no válida.",
            "Elenco di file non valido.",
            "Lista de ficheiros inválida.",
        ],
    ),
    (
        "Invalid helper.",
        [
            "Ungültiges Hilfsprogramm.",
            "Asistente no válido.",
            "Assistente non valido.",
            "Assistente inválido.",
        ],
    ),
    (
        "Invalid number of update files.",
        [
            "Ungültige Anzahl an Updatedateien.",
            "Número de archivos de actualización no válido.",
            "Numero di file di aggiornamento non valido.",
            "Número de ficheiros de atualização inválido.",
        ],
    ),
    (
        "Invalid player filename.",
        [
            "Ungültiger Dateiname des Players.",
            "Nombre del archivo del reproductor no válido.",
            "Nome del file del lettore non valido.",
            "Nome do ficheiro do leitor inválido.",
        ],
    ),
    (
        "Invalid process.",
        [
            "Ungültiger Prozess.",
            "Proceso no válido.",
            "Processo non valido.",
            "Processo inválido.",
        ],
    ),
    (
        "Invalid restart executable.",
        [
            "Ungültige ausführbare Datei für den Neustart.",
            "Ejecutable de reinicio no válido.",
            "Eseguibile di riavvio non valido.",
            "Executável de reinício inválido.",
        ],
    ),
    (
        "Invalid staging directory.",
        [
            "Ungültiger Vorbereitungsordner.",
            "Carpeta de preparación no válida.",
            "Cartella di preparazione non valida.",
            "Pasta de preparação inválida.",
        ],
    ),
    (
        "Invalid update archive.",
        [
            "Ungültiges Updatearchiv.",
            "Archivo de actualización no válido.",
            "Archivio di aggiornamento non valido.",
            "Arquivo de atualização inválido.",
        ],
    ),
    (
        "Invalid update confirmation.",
        [
            "Ungültige Updatebestätigung.",
            "Confirmación de actualización no válida.",
            "Conferma di aggiornamento non valida.",
            "Confirmação de atualização inválida.",
        ],
    ),
    (
        "Invalid update helper.",
        [
            "Ungültiges Update-Hilfsprogramm.",
            "Asistente de actualización no válido.",
            "Assistente di aggiornamento non valido.",
            "Assistente de atualização inválido.",
        ],
    ),
    (
        "Invalid update manifest.",
        [
            "Ungültiges Updatemanifest.",
            "Manifiesto de actualización no válido.",
            "Manifesto di aggiornamento non valido.",
            "Manifesto de atualização inválido.",
        ],
    ),
    (
        "Invalid update path.",
        [
            "Ungültiger Updatepfad.",
            "Ruta de actualización no válida.",
            "Percorso di aggiornamento non valido.",
            "Caminho de atualização inválido.",
        ],
    ),
    (
        "Reparse point refused.",
        [
            "Analysepunkt abgelehnt.",
            "Punto de reanálisis rechazado.",
            "Punto di analisi rifiutato.",
            "Ponto de nova análise recusado.",
        ],
    ),
    (
        "Rollback incomplete:",
        [
            "Wiederherstellung unvollständig:",
            "Restauración incompleta:",
            "Ripristino incompleto:",
            "Restauro incompleto:",
        ],
    ),
    (
        "Staging link refused.",
        [
            "Verknüpfung zum Vorbereitungsordner abgelehnt.",
            "Enlace a la carpeta de preparación rechazado.",
            "Collegamento alla cartella di preparazione rifiutato.",
            "Ligação à pasta de preparação recusada.",
        ],
    ),
    (
        "Symbolic link refused.",
        [
            "Symbolischer Link abgelehnt.",
            "Enlace simbólico rechazado.",
            "Collegamento simbolico rifiutato.",
            "Ligação simbólica recusada.",
        ],
    ),
    (
        "The GitHub release list is invalid",
        [
            "Die Versionsliste von GitHub ist ungültig",
            "La lista de versiones de GitHub no es válida",
            "L'elenco delle versioni di GitHub non è valido",
            "A lista de versões do GitHub é inválida",
        ],
    ),
    (
        "The GitHub response exceeds the allowed size",
        [
            "Die Antwort von GitHub überschreitet die zulässige Größe",
            "La respuesta de GitHub supera el tamaño permitido",
            "La risposta di GitHub supera la dimensione consentita",
            "A resposta do GitHub excede o tamanho permitido",
        ],
    ),
    (
        "The archive SHA-256 checksum is incorrect",
        [
            "Die SHA-256-Prüfsumme des Archivs ist falsch",
            "La suma de comprobación SHA-256 del archivo es incorrecta",
            "Il codice di controllo SHA-256 dell'archivio è errato",
            "A soma de verificação SHA-256 do arquivo está incorreta",
        ],
    ),
    (
        "The archive contains an unauthorized link or file.",
        [
            "Das Archiv enthält einen unzulässigen Link oder eine unzulässige Datei.",
            "El archivo contiene un enlace o archivo no permitido.",
            "L'archivio contiene un collegamento o un file non consentito.",
            "O arquivo contém uma ligação ou um ficheiro não autorizado.",
        ],
    ),
    (
        "The archive contents are corrupted.",
        [
            "Der Inhalt des Archivs ist beschädigt.",
            "El contenido del archivo está dañado.",
            "Il contenuto dell'archivio è danneggiato.",
            "O conteúdo do arquivo está corrompido.",
        ],
    ),
    (
        "The archive exceeds its advertised size",
        [
            "Das Archiv ist größer als angegeben",
            "El archivo supera el tamaño anunciado",
            "L'archivio supera la dimensione dichiarata",
            "O arquivo excede o tamanho indicado",
        ],
    ),
    (
        "The archive is incomplete or encrypted.",
        [
            "Das Archiv ist unvollständig oder verschlüsselt.",
            "El archivo está incompleto o cifrado.",
            "L'archivio è incompleto o cifrato.",
            "O arquivo está incompleto ou encriptado.",
        ],
    ),
    (
        "The archive is too large.",
        [
            "Das Archiv ist zu groß.",
            "El archivo es demasiado grande.",
            "L'archivio è troppo grande.",
            "O arquivo é demasiado grande.",
        ],
    ),
    (
        "The archive size differs from its advertised size",
        [
            "Die Archivgröße weicht von der Angabe ab",
            "El tamaño del archivo no coincide con el anunciado",
            "La dimensione dell'archivio non corrisponde a quella dichiarata",
            "O tamanho do arquivo difere do indicado",
        ],
    ),
    (
        "The confirmation file is too large.",
        [
            "Die Bestätigungsdatei ist zu groß.",
            "El archivo de confirmación es demasiado grande.",
            "Il file di conferma è troppo grande.",
            "O ficheiro de confirmação é demasiado grande.",
        ],
    ),
    (
        "The destination is inaccessible",
        [
            "Der Zielpfad ist nicht zugänglich",
            "El destino es inaccesible",
            "La destinazione non è accessibile",
            "O destino está inacessível",
        ],
    ),
    (
        "The directory is inaccessible",
        [
            "Der Ordner ist nicht zugänglich",
            "La carpeta es inaccesible",
            "La cartella non è accessibile",
            "A pasta está inacessível",
        ],
    ),
    (
        "The download is incomplete",
        [
            "Der Download ist unvollständig",
            "La descarga está incompleta",
            "Il download è incompleto",
            "A transferência está incompleta",
        ],
    ),
    (
        "The downloaded archive is inaccessible.",
        [
            "Das heruntergeladene Archiv ist nicht zugänglich.",
            "No se puede acceder al archivo descargado.",
            "L'archivio scaricato non è accessibile.",
            "O arquivo transferido está inacessível.",
        ],
    ),
    (
        "The executable is inaccessible",
        [
            "Die ausführbare Datei ist nicht zugänglich",
            "El ejecutable es inaccesible",
            "L'eseguibile non è accessibile",
            "O executável está inacessível",
        ],
    ),
    (
        "The executable is missing from the manifest.",
        [
            "Die ausführbare Datei fehlt im Manifest.",
            "El ejecutable no figura en el manifiesto.",
            "L'eseguibile manca nel manifesto.",
            "O executável está em falta no manifesto.",
        ],
    ),
    (
        "The executable is unreadable.",
        [
            "Die ausführbare Datei ist nicht lesbar.",
            "No se puede leer el ejecutable.",
            "L'eseguibile è illeggibile.",
            "O executável está ilegível.",
        ],
    ),
    (
        "The file is inaccessible",
        [
            "Die Datei ist nicht zugänglich",
            "El archivo es inaccesible",
            "Il file non è accessibile",
            "O ficheiro está inacessível",
        ],
    ),
    (
        "The file is unreadable",
        [
            "Die Datei ist nicht lesbar",
            "El archivo es ilegible",
            "Il file è illeggibile",
            "O ficheiro está ilegível",
        ],
    ),
    (
        "The installed version has changed; start the update again.",
        [
            "Die installierte Version hat sich geändert; starten Sie das Update erneut.",
            "La versión instalada ha cambiado; inicie la actualización de nuevo.",
            "La versione installata è cambiata; avviare nuovamente l'aggiornamento.",
            "A versão instalada mudou; inicie a atualização novamente.",
        ],
    ),
    (
        "The package executable is missing.",
        [
            "Die ausführbare Datei fehlt im Paket.",
            "Falta el ejecutable del paquete.",
            "L'eseguibile del pacchetto è mancante.",
            "O executável está em falta no pacote.",
        ],
    ),
    (
        "The player did not close in time.",
        [
            "Der Player wurde nicht rechtzeitig beendet.",
            "El reproductor no se cerró a tiempo.",
            "Il lettore non si è chiuso in tempo.",
            "O leitor não fechou a tempo.",
        ],
    ),
    (
        "The player did not confirm startup.",
        [
            "Der Player hat seinen Start nicht bestätigt.",
            "El reproductor no confirmó el inicio.",
            "Il lettore non ha confermato l'avvio.",
            "O leitor não confirmou o arranque.",
        ],
    ),
    (
        "The prepared package has changed.",
        [
            "Das vorbereitete Paket wurde verändert.",
            "El paquete preparado ha cambiado.",
            "Il pacchetto preparato è cambiato.",
            "O pacote preparado foi alterado.",
        ],
    ),
    (
        "The update check was interrupted. Try again.",
        [
            "Die Updatesuche wurde unterbrochen. Versuchen Sie es erneut.",
            "Se interrumpió la búsqueda de actualizaciones. Inténtelo de nuevo.",
            "La ricerca di aggiornamenti è stata interrotta. Riprovare.",
            "A procura de atualizações foi interrompida. Tente novamente.",
        ],
    ),
    (
        "The update contains a duplicate file.",
        [
            "Das Update enthält eine doppelte Datei.",
            "La actualización contiene un archivo duplicado.",
            "L'aggiornamento contiene un file duplicato.",
            "A atualização contém um ficheiro duplicado.",
        ],
    ),
    (
        "The update contains an invalid filename.",
        [
            "Das Update enthält einen ungültigen Dateinamen.",
            "La actualización contiene un nombre de archivo no válido.",
            "L'aggiornamento contiene un nome di file non valido.",
            "A atualização contém um nome de ficheiro inválido.",
        ],
    ),
    (
        "The update contains an unauthorized path.",
        [
            "Das Update enthält einen unzulässigen Pfad.",
            "La actualización contiene una ruta no permitida.",
            "L'aggiornamento contiene un percorso non consentito.",
            "A atualização contém um caminho não autorizado.",
        ],
    ),
    (
        "The update directory is missing.",
        [
            "Der Updateordner fehlt.",
            "Falta la carpeta de actualización.",
            "La cartella di aggiornamento è mancante.",
            "A pasta de atualização está em falta.",
        ],
    ),
    (
        "The update executable is missing.",
        [
            "Die ausführbare Datei fehlt im Update.",
            "Falta el ejecutable de la actualización.",
            "L'eseguibile dell'aggiornamento è mancante.",
            "O executável está em falta na atualização.",
        ],
    ),
    (
        "The update failed. Diagnostic",
        [
            "Das Update ist fehlgeschlagen. Diagnose",
            "La actualización ha fallado. Diagnóstico",
            "L'aggiornamento non è riuscito. Diagnostica",
            "A atualização falhou. Diagnóstico",
        ],
    ),
    (
        "The update failed; the previous version was restored.",
        [
            "Das Update ist fehlgeschlagen; die vorherige Version wurde wiederhergestellt.",
            "La actualización ha fallado; se ha restaurado la versión anterior.",
            "L'aggiornamento non è riuscito; la versione precedente è stata ripristinata.",
            "A atualização falhou; a versão anterior foi restaurada.",
        ],
    ),
    (
        "The update lock is inaccessible",
        [
            "Die Updatesperre ist nicht zugänglich",
            "El bloqueo de actualización es inaccesible",
            "Il blocco dell'aggiornamento non è accessibile",
            "O bloqueio de atualização está inacessível",
        ],
    ),
    (
        "The update process stopped.",
        [
            "Der Updateprozess wurde beendet.",
            "El proceso de actualización se ha detenido.",
            "Il processo di aggiornamento si è arrestato.",
            "O processo de atualização parou.",
        ],
    ),
    (
        "This directory is not writable. Move Spotiamp to your user directory.",
        [
            "In diesen Ordner kann nicht geschrieben werden. Verschieben Sie Spotiamp in Ihren Benutzerordner.",
            "No se puede escribir en esta carpeta. Mueva Spotiamp a su carpeta de usuario.",
            "Questa cartella non è scrivibile. Spostare Spotiamp nella propria cartella utente.",
            "Não é possível escrever nesta pasta. Mova o Spotiamp para a sua pasta de utilizador.",
        ],
    ),
    (
        "Unexpected file type.",
        [
            "Unerwarteter Dateityp.",
            "Tipo de archivo inesperado.",
            "Tipo di file inatteso.",
            "Tipo de ficheiro inesperado.",
        ],
    ),
    (
        "Update cancelled; previous version restored.",
        [
            "Update abgebrochen; vorherige Version wiederhergestellt.",
            "Actualización cancelada; se ha restaurado la versión anterior.",
            "Aggiornamento annullato; versione precedente ripristinata.",
            "Atualização cancelada; versão anterior restaurada.",
        ],
    ),
    (
        "Update cleanup refused.",
        [
            "Bereinigung der Updatedateien abgelehnt.",
            "Limpieza de la actualización rechazada.",
            "Pulizia dell'aggiornamento rifiutata.",
            "Limpeza da atualização recusada.",
        ],
    ),
    (
        "Update confirmation is missing.",
        [
            "Die Updatebestätigung fehlt.",
            "Falta la confirmación de actualización.",
            "La conferma di aggiornamento è mancante.",
            "A confirmação de atualização está em falta.",
        ],
    ),
    (
        "Update confirmation timed out.",
        [
            "Zeitüberschreitung bei der Updatebestätigung.",
            "Se agotó el tiempo de confirmación de la actualización.",
            "Tempo scaduto per la conferma dell'aggiornamento.",
            "O tempo de confirmação da atualização expirou.",
        ],
    ),
    (
        "Update failed:",
        [
            "Update fehlgeschlagen:",
            "Actualización fallida:",
            "Aggiornamento non riuscito:",
            "A atualização falhou:",
        ],
    ),
    (
        "Updates are unavailable for this platform",
        [
            "Für diese Plattform sind keine Updates verfügbar",
            "Las actualizaciones no están disponibles para esta plataforma",
            "Gli aggiornamenti non sono disponibili per questa piattaforma",
            "As atualizações não estão disponíveis para esta plataforma",
        ],
    ),
    (
        "Updates are unavailable for this system.",
        [
            "Für dieses System sind keine Updates verfügbar.",
            "Las actualizaciones no están disponibles para este sistema.",
            "Gli aggiornamenti non sono disponibili per questo sistema.",
            "As atualizações não estão disponíveis para este sistema.",
        ],
    ),
    (
        "Version information is invalid.",
        [
            "Die Versionsinformationen sind ungültig.",
            "La información de versión no es válida.",
            "Le informazioni sulla versione non sono valide.",
            "A informação da versão é inválida.",
        ],
    ),
    (
        "Version information is missing.",
        [
            "Die Versionsinformationen fehlen.",
            "Falta la información de versión.",
            "Le informazioni sulla versione sono mancanti.",
            "A informação da versão está em falta.",
        ],
    ),
    (
        "Version information is too large.",
        [
            "Die Versionsinformationen sind zu umfangreich.",
            "La información de versión es demasiado grande.",
            "Le informazioni sulla versione sono troppo grandi.",
            "A informação da versão é demasiado grande.",
        ],
    ),
    (
        "Version information is unreadable.",
        [
            "Die Versionsinformationen sind nicht lesbar.",
            "No se puede leer la información de versión.",
            "Le informazioni sulla versione sono illeggibili.",
            "A informação da versão está ilegível.",
        ],
    ),
];
