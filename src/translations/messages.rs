//! Application messages. English keys, translations ordered DE, ES, IT, PT.

pub(super) const ENTRIES: &[(&str, [&str; 4])] = &[
    (
        "Adding to the Spotify playlist...",
        [
            "Titel werden zur Spotify-Playlist hinzugefügt...",
            "Añadiendo a la playlist de Spotify...",
            "Aggiunta alla playlist Spotify...",
            "A adicionar à playlist Spotify...",
        ],
    ),
    (
        "Addition in progress; wait for confirmation",
        [
            "Hinzufügen läuft; Bestätigung abwarten",
            "Adición en curso; espera la confirmación",
            "Aggiunta in corso; attendi la conferma",
            "Adição em curso; aguarda a confirmação",
        ],
    ),
    (
        "Addition unconfirmed; check the playlist before retrying",
        [
            "Hinzufügen unbestätigt; Playlist vor erneutem Versuch prüfen",
            "Adición sin confirmar; comprueba la playlist antes de reintentar",
            "Aggiunta non confermata; controlla la playlist prima di riprovare",
            "Adição não confirmada; verifica a playlist antes de tentar novamente",
        ],
    ),
    (
        "Application closed",
        [
            "Anwendung geschlossen",
            "Aplicación cerrada",
            "Applicazione chiusa",
            "Aplicação fechada",
        ],
    ),
    (
        "Audio output closed",
        [
            "Audioausgabe geschlossen",
            "Salida de audio cerrada",
            "Uscita audio chiusa",
            "Saída de áudio fechada",
        ],
    ),
    (
        "Audio output stopped responding; check your device",
        [
            "Audioausgabe reagiert nicht mehr; Gerät prüfen",
            "La salida de audio no responde; comprueba el dispositivo",
            "L'uscita audio non risponde; controlla il dispositivo",
            "A saída de áudio não responde; verifica o dispositivo",
        ],
    ),
    (
        "Audio output unavailable. Check the device, then reconnect.",
        [
            "Audioausgabe nicht verfügbar. Gerät prüfen und erneut verbinden.",
            "Salida de audio no disponible. Comprueba el dispositivo y vuelve a conectar.",
            "Uscita audio non disponibile. Controlla il dispositivo e riconnettiti.",
            "Saída de áudio indisponível. Verifica o dispositivo e volta a ligar.",
        ],
    ),
    (
        "Cannot copy: clipboard unavailable.",
        [
            "Kopieren nicht möglich: Zwischenablage nicht verfügbar.",
            "No se puede copiar: portapapeles no disponible.",
            "Impossibile copiare: appunti non disponibili.",
            "Não é possível copiar: área de transferência indisponível.",
        ],
    ),
    (
        "Cannot launch the browser.",
        [
            "Browser kann nicht gestartet werden.",
            "No se puede abrir el navegador.",
            "Impossibile avviare il browser.",
            "Não é possível abrir o navegador.",
        ],
    ),
    (
        "Cannot open the Spotify library.",
        [
            "Spotify-Bibliothek kann nicht geöffnet werden.",
            "No se puede abrir la biblioteca de Spotify.",
            "Impossibile aprire la libreria Spotify.",
            "Não é possível abrir a biblioteca do Spotify.",
        ],
    ),
    (
        "Cannot start opening the project page.",
        [
            "Projektseite kann nicht geöffnet werden.",
            "No se puede iniciar la apertura de la página del proyecto.",
            "Impossibile avviare l'apertura della pagina del progetto.",
            "Não é possível iniciar a abertura da página do projeto.",
        ],
    ),
    (
        "Clearing the credential store...",
        [
            "Anmeldedaten werden gelöscht...",
            "Borrando las credenciales guardadas...",
            "Eliminazione delle credenziali salvate...",
            "A apagar as credenciais guardadas...",
        ],
    ),
    (
        "Connect Spotify Premium to get started",
        [
            "Zum Starten Spotify Premium verbinden",
            "Conecta Spotify Premium para empezar",
            "Collega Spotify Premium per iniziare",
            "Liga o Spotify Premium para começar",
        ],
    ),
    (
        "Connect Spotify before adding a link",
        [
            "Vor dem Hinzufügen eines Links Spotify verbinden",
            "Conecta Spotify antes de añadir un enlace",
            "Collega Spotify prima di aggiungere un link",
            "Liga o Spotify antes de adicionar uma ligação",
        ],
    ),
    (
        "Connect Spotify to edit a playlist",
        [
            "Spotify verbinden, um eine Playlist zu bearbeiten",
            "Conecta Spotify para editar una playlist",
            "Connetti Spotify per modificare una playlist",
            "Liga o Spotify para editar uma playlist",
        ],
    ),
    (
        "Connect Spotify to get started.",
        [
            "Zum Starten Spotify verbinden.",
            "Conecta Spotify para empezar.",
            "Collega Spotify per iniziare.",
            "Liga o Spotify para começar.",
        ],
    ),
    (
        "Connect Spotify to load this playlist",
        [
            "Spotify verbinden, um diese Playlist zu laden",
            "Conecta Spotify para cargar esta playlist",
            "Collega Spotify per caricare questa playlist",
            "Liga o Spotify para carregar esta playlist",
        ],
    ),
    (
        "Connect Spotify to play the queue",
        [
            "Spotify verbinden, um die Warteschlange abzuspielen",
            "Conecta Spotify para reproducir la cola",
            "Collega Spotify per riprodurre la coda",
            "Liga o Spotify para reproduzir a fila",
        ],
    ),
    (
        "Connect Spotify to play this playlist",
        [
            "Spotify verbinden, um diese Playlist abzuspielen",
            "Conecta Spotify para reproducir esta playlist",
            "Collega Spotify per riprodurre questa playlist",
            "Liga o Spotify para reproduzir esta playlist",
        ],
    ),
    (
        "Connect Spotify to play this playlist; the queue was retained",
        [
            "Spotify zum Abspielen verbinden; Warteschlange beibehalten",
            "Conecta Spotify para reproducir esta playlist; se conservó la cola",
            "Collega Spotify per riprodurre la playlist; coda conservata",
            "Liga o Spotify para reproduzir esta playlist; a fila foi mantida",
        ],
    ),
    (
        "Connect Spotify to play this track",
        [
            "Spotify verbinden, um diesen Titel abzuspielen",
            "Conecta Spotify para reproducir esta canción",
            "Collega Spotify per riprodurre questo brano",
            "Liga o Spotify para reproduzir esta música",
        ],
    ),
    (
        "Connect Spotify to search for tracks",
        [
            "Spotify verbinden, um Titel zu suchen",
            "Conecta Spotify para buscar canciones",
            "Collega Spotify per cercare brani",
            "Liga o Spotify para pesquisar músicas",
        ],
    ),
    (
        "Connect Spotify to view your playlists",
        [
            "Spotify verbinden, um deine Playlists anzuzeigen",
            "Conecta Spotify para ver tus playlists",
            "Collega Spotify per vedere le tue playlist",
            "Liga o Spotify para ver as tuas playlists",
        ],
    ),
    (
        "Connected. Paste a track, album or playlist link.",
        [
            "Verbunden. Link zu Titel, Album oder Playlist einfügen.",
            "Conectado. Pega un enlace de canción, álbum o playlist.",
            "Connesso. Incolla un link a un brano, album o playlist.",
            "Ligado. Cola uma ligação de faixa, álbum ou playlist.",
        ],
    ),
    (
        "Connected. You can close this tab and return to Spotiamp.",
        [
            "Verbunden. Du kannst diesen Tab schließen und zu Spotiamp zurückkehren.",
            "Conectado. Puedes cerrar esta pestaña y volver a Spotiamp.",
            "Connesso. Puoi chiudere questa scheda e tornare a Spotiamp.",
            "Ligado. Podes fechar este separador e voltar ao Spotiamp.",
        ],
    ),
    (
        "Connected; saving is unconfirmed, automatic reconnection is not guaranteed",
        [
            "Verbunden; Speicherung unbestätigt, automatische Anmeldung nicht garantiert",
            "Conectado; guardado sin confirmar, reconexión automática no garantizada",
            "Connesso; salvataggio non confermato, riconnessione automatica non garantita",
            "Ligado; gravação não confirmada, ligação automática não garantida",
        ],
    ),
    (
        "Connecting to Spotify: authorize access in your browser if prompted",
        [
            "Spotify-Verbindung: Zugriff bei Aufforderung im Browser erlauben",
            "Conectando con Spotify: autoriza el acceso en el navegador si se solicita",
            "Connessione a Spotify: autorizza l'accesso nel browser se richiesto",
            "A ligar ao Spotify: autoriza o acesso no navegador se for pedido",
        ],
    ),
    (
        "Connection cancelled",
        [
            "Verbindung abgebrochen",
            "Conexión cancelada",
            "Connessione annullata",
            "Ligação cancelada",
        ],
    ),
    (
        "Connection cancelled or denied. Close the tab and try again; local port 5588 must be available.",
        [
            "Verbindung abgebrochen oder verweigert. Tab schließen und erneut versuchen; lokaler Port 5588 muss frei sein.",
            "Conexión cancelada o denegada. Cierra la pestaña y reintenta; el puerto local 5588 debe estar libre.",
            "Connessione annullata o negata. Chiudi la scheda e riprova; la porta locale 5588 deve essere libera.",
            "Ligação cancelada ou recusada. Fecha o separador e tenta de novo; a porta local 5588 tem de estar livre.",
        ],
    ),
    (
        "Connection cancelled; saved session retained",
        [
            "Verbindung abgebrochen; gespeicherte Sitzung beibehalten",
            "Conexión cancelada; sesión guardada conservada",
            "Connessione annullata; sessione salvata conservata",
            "Ligação cancelada; sessão guardada mantida",
        ],
    ),
    (
        "Connection lost. Click Connect to resume.",
        [
            "Verbindung verloren. Zum Fortfahren auf Verbinden klicken.",
            "Conexión perdida. Pulsa Conectar para continuar.",
            "Connessione persa. Premi Connetti per riprendere.",
            "Ligação perdida. Clica em Ligar para retomar.",
        ],
    ),
    (
        "Connection timed out after 3 minutes; click Connect to try again",
        [
            "Verbindung nach 3 Minuten abgelaufen; zum Wiederholen auf Verbinden klicken",
            "Conexión agotada tras 3 minutos; pulsa Conectar para reintentar",
            "Connessione scaduta dopo 3 minuti; premi Connetti per riprovare",
            "Ligação expirada após 3 minutos; clica em Ligar para tentar de novo",
        ],
    ),
    (
        "Could not connect to Spotify. Check your network and Premium subscription.",
        [
            "Spotify-Verbindung fehlgeschlagen. Netzwerk und Premium-Abo prüfen.",
            "No se pudo conectar con Spotify. Comprueba la red y tu suscripción Premium.",
            "Impossibile connettersi a Spotify. Controlla la rete e l'abbonamento Premium.",
            "Não foi possível ligar ao Spotify. Verifica a rede e a subscrição Premium.",
        ],
    ),
    (
        "Could not create the audio thread",
        [
            "Audio-Thread konnte nicht erstellt werden",
            "No se pudo crear el hilo de audio",
            "Impossibile creare il thread audio",
            "Não foi possível criar o thread de áudio",
        ],
    ),
    (
        "Could not initialize audio volume",
        [
            "Lautstärke konnte nicht initialisiert werden",
            "No se pudo inicializar el volumen",
            "Impossibile inizializzare il volume audio",
            "Não foi possível iniciar o volume de áudio",
        ],
    ),
    (
        "Could not open the project page in your browser.",
        [
            "Projektseite konnte im Browser nicht geöffnet werden.",
            "No se pudo abrir la página del proyecto en el navegador.",
            "Impossibile aprire la pagina del progetto nel browser.",
            "Não foi possível abrir a página do projeto no navegador.",
        ],
    ),
    (
        "Could not prepare the Spotify connection",
        [
            "Spotify-Verbindung konnte nicht vorbereitet werden",
            "No se pudo preparar la conexión con Spotify",
            "Impossibile preparare la connessione a Spotify",
            "Não foi possível preparar a ligação ao Spotify",
        ],
    ),
    (
        "Could not read this Spotify link; check its access",
        [
            "Spotify-Link konnte nicht gelesen werden; Zugriffsrechte prüfen",
            "No se pudo leer este enlace de Spotify; comprueba el acceso",
            "Impossibile leggere questo link Spotify; controlla l'accesso",
            "Não foi possível ler esta ligação do Spotify; verifica o acesso",
        ],
    ),
    (
        "Could not reconnect; session retained. Click Connect to try again.",
        [
            "Erneute Verbindung fehlgeschlagen; Sitzung beibehalten. Auf Verbinden klicken.",
            "No se pudo reconectar; sesión conservada. Pulsa Conectar para reintentar.",
            "Riconnessione non riuscita; sessione conservata. Premi Connetti per riprovare.",
            "Não foi possível voltar a ligar; sessão mantida. Clica em Ligar para tentar de novo.",
        ],
    ),
    (
        "Could not remove the saved session; unlock the credential store and try again",
        [
            "Gespeicherte Sitzung konnte nicht gelöscht werden; Anmeldespeicher entsperren und erneut versuchen",
            "No se pudo borrar la sesión guardada; desbloquea el almacén de credenciales y reintenta",
            "Impossibile eliminare la sessione salvata; sblocca l'archivio credenziali e riprova",
            "Não foi possível apagar a sessão guardada; desbloqueia o gestor de credenciais e tenta de novo",
        ],
    ),
    (
        "Could not retrieve your Spotify playlists",
        [
            "Spotify-Playlists konnten nicht geladen werden",
            "No se pudieron obtener tus playlists de Spotify",
            "Impossibile recuperare le tue playlist Spotify",
            "Não foi possível obter as tuas playlists do Spotify",
        ],
    ),
    (
        "Could not start the audio engine",
        [
            "Audio-Engine konnte nicht gestartet werden",
            "No se pudo iniciar el motor de audio",
            "Impossibile avviare il motore audio",
            "Não foi possível iniciar o motor de áudio",
        ],
    ),
    (
        "Could not verify the playlist; no tracks added",
        [
            "Playlist konnte nicht geprüft werden; keine Titel hinzugefügt",
            "No se pudo verificar la playlist; no se añadieron canciones",
            "Impossibile verificare la playlist; nessun brano aggiunto",
            "Não foi possível verificar a playlist; nenhuma faixa adicionada",
        ],
    ),
    (
        "Could not verify the playlist; no tracks moved",
        [
            "Playlist konnte nicht geprüft werden; keine Titel verschoben",
            "No se pudo verificar la playlist; no se movieron canciones",
            "Impossibile verificare la playlist; nessun brano spostato",
            "Não foi possível verificar a playlist; nenhuma faixa movida",
        ],
    ),
    (
        "Could not verify your playlists; no tracks added",
        [
            "Deine Playlists konnten nicht geprüft werden; keine Titel hinzugefügt",
            "No se pudieron comprobar tus playlists; no se añadieron canciones",
            "Impossibile verificare le tue playlist; nessun brano aggiunto",
            "Não foi possível verificar as tuas playlists; nenhuma faixa adicionada",
        ],
    ),
    (
        "Creating the Spotify playlist...",
        [
            "Spotify-Playlist wird erstellt...",
            "Creando la playlist de Spotify...",
            "Creazione della playlist Spotify...",
            "A criar a playlist Spotify...",
        ],
    ),
    (
        "Creation or addition unconfirmed; check your playlists before retrying",
        [
            "Erstellung oder Hinzufügen unbestätigt; zuerst Playlists prüfen",
            "Creación o adición sin confirmar; comprueba tus playlists antes de reintentar",
            "Creazione o aggiunta non confermata; controlla le playlist prima di riprovare",
            "Criação ou adição não confirmada; verifica as playlists antes de tentar novamente",
        ],
    ),
    (
        "Creation unconfirmed; check your playlists before retrying",
        [
            "Erstellung unbestätigt; Playlists vor erneutem Versuch prüfen",
            "Creación sin confirmar; comprueba tus playlists antes de reintentar",
            "Creazione non confermata; controlla le playlist prima di riprovare",
            "Criação não confirmada; verifica as playlists antes de tentar novamente",
        ],
    ),
    (
        "Credential store locked or unavailable",
        [
            "Anmeldespeicher gesperrt oder nicht verfügbar",
            "Almacén de credenciales bloqueado o no disponible",
            "Archivio credenziali bloccato o non disponibile",
            "Gestor de credenciais bloqueado ou indisponível",
        ],
    ),
    (
        "Credential store unavailable",
        [
            "Anmeldespeicher nicht verfügbar",
            "Almacén de credenciales no disponible",
            "Archivio credenziali non disponibile",
            "Gestor de credenciais indisponível",
        ],
    ),
    (
        "Credential store unavailable: signed in for this session only",
        [
            "Anmeldespeicher nicht verfügbar: nur für diese Sitzung angemeldet",
            "Almacén de credenciales no disponible: conexión solo para esta sesión",
            "Archivio credenziali non disponibile: accesso solo per questa sessione",
            "Gestor de credenciais indisponível: ligação apenas nesta sessão",
        ],
    ),
    (
        "Disconnected; clearing the credential store...",
        [
            "Getrennt; Anmeldedaten werden gelöscht...",
            "Desconectado; borrando credenciales guardadas...",
            "Disconnesso; eliminazione delle credenziali salvate...",
            "Desligado; a apagar as credenciais guardadas...",
        ],
    ),
    (
        "Disconnected; credentials removed from the credential store",
        [
            "Getrennt; gespeicherte Anmeldedaten gelöscht",
            "Desconectado; credenciales eliminadas del almacén",
            "Disconnesso; credenziali eliminate dall'archivio",
            "Desligado; credenciais apagadas do gestor",
        ],
    ),
    (
        "End of queue",
        [
            "Ende der Warteschlange",
            "Fin de la cola",
            "Fine della coda",
            "Fim da fila",
        ],
    ),
    (
        "Expected URI: spotify:track|album|playlist:identifier",
        [
            "URI erwartet: spotify:track|album|playlist:Kennung",
            "URI esperado: spotify:track|album|playlist:identificador",
            "URI previsto: spotify:track|album|playlist:identificatore",
            "URI esperado: spotify:track|album|playlist:identificador",
        ],
    ),
    (
        "Expected link: https://open.spotify.com/track|album|playlist/identifier",
        [
            "Link erwartet: https://open.spotify.com/track|album|playlist/Kennung",
            "Enlace esperado: https://open.spotify.com/track|album|playlist/identificador",
            "Link previsto: https://open.spotify.com/track|album|playlist/identificatore",
            "Ligação esperada: https://open.spotify.com/track|album|playlist/identificador",
        ],
    ),
    (
        "Finish the current library action, then try again.",
        [
            "Schließe die aktuelle Aktion in der Bibliothek ab und versuche es erneut.",
            "Termina la acción actual en la biblioteca y vuelve a intentarlo.",
            "Completa l’azione in corso nella libreria e riprova.",
            "Termina a ação atual na biblioteca e tenta novamente.",
        ],
    ),
    (
        "Import in progress; Clear cancels it",
        [
            "Import läuft; Leeren bricht ihn ab",
            "Importación en curso; Vaciar la cancela",
            "Importazione in corso; Svuota la annulla",
            "Importação em curso; Limpar cancela-a",
        ],
    ),
    (
        "Importing links; the queue was retained",
        [
            "Links werden importiert; Warteschlange beibehalten",
            "Importando enlaces; se conservó la cola",
            "Importazione link; coda conservata",
            "A importar ligações; a fila foi mantida",
        ],
    ),
    (
        "Importing links; wait for completion or clear the queue",
        [
            "Links werden importiert; warten oder Warteschlange leeren",
            "Importando enlaces; espera o vacía la cola",
            "Importazione link; attendi o svuota la coda",
            "A importar ligações; aguarda ou limpa a fila",
        ],
    ),
    (
        "Inconsistent Spotify library length",
        [
            "Unstimmige Länge der Spotify-Bibliothek",
            "Tamaño incoherente de la biblioteca de Spotify",
            "Dimensione della libreria Spotify incoerente",
            "Tamanho inconsistente da biblioteca do Spotify",
        ],
    ),
    (
        "Invalid Spotify identifier",
        [
            "Ungültige Spotify-Kennung",
            "Identificador de Spotify no válido",
            "Identificatore Spotify non valido",
            "Identificador do Spotify inválido",
        ],
    ),
    (
        "Invalid Spotify library length",
        [
            "Ungültige Länge der Spotify-Bibliothek",
            "Tamaño de biblioteca de Spotify no válido",
            "Dimensione della libreria Spotify non valida",
            "Tamanho da biblioteca do Spotify inválido",
        ],
    ),
    (
        "Invalid Spotify playlist identifier",
        [
            "Ungültige Spotify-Playlist-Kennung",
            "Identificador de playlist de Spotify no válido",
            "Identificatore playlist Spotify non valido",
            "Identificador de playlist do Spotify inválido",
        ],
    ),
    (
        "Invalid insertion position; the queue was retained",
        [
            "Ungültige Einfügeposition; Warteschlange unverändert",
            "Posición de inserción no válida; se conservó la cola",
            "Posizione d'inserimento non valida; la coda è stata conservata",
            "Posição de inserção inválida; a fila foi mantida",
        ],
    ),
    (
        "Invalid playlist name (1 to 100 characters)",
        [
            "Ungültiger Playlistname (1 bis 100 Zeichen)",
            "Nombre de playlist no válido (de 1 a 100 caracteres)",
            "Nome playlist non valido (da 1 a 100 caratteri)",
            "Nome de playlist inválido (1 a 100 caracteres)",
        ],
    ),
    (
        "Invalid selection; no tracks added",
        [
            "Ungültige Auswahl; keine Titel hinzugefügt",
            "Selección no válida; no se añadieron canciones",
            "Selezione non valida; nessun brano aggiunto",
            "Seleção inválida; nenhuma faixa adicionada",
        ],
    ),
    (
        "Invalid selection; no tracks moved",
        [
            "Ungültige Auswahl; keine Titel verschoben",
            "Selección no válida; no se movieron canciones",
            "Selezione non valida; nessun brano spostato",
            "Seleção inválida; nenhuma faixa movida",
        ],
    ),
    (
        "Invalid selection; no tracks removed",
        [
            "Ungültige Auswahl; keine Titel entfernt",
            "Selección no válida; no se quitó ningún tema",
            "Selezione non valida; nessun brano rimosso",
            "Seleção inválida; nenhuma faixa removida",
        ],
    ),
    (
        "Language applied but not saved: cannot write settings.",
        [
            "Sprache angewendet, aber nicht gespeichert: Schreibfehler.",
            "Idioma aplicado pero no guardado: no se pudo escribir la configuración.",
            "Lingua applicata ma non salvata: impossibile scrivere le impostazioni.",
            "Idioma aplicado mas não guardado: não foi possível gravar as definições.",
        ],
    ),
    (
        "Language applied but not saved: settings folder unavailable.",
        [
            "Sprache angewendet, aber nicht gespeichert: Einstellungsordner nicht verfügbar.",
            "Idioma aplicado pero no guardado: carpeta de configuración no disponible.",
            "Lingua applicata ma non salvata: cartella delle impostazioni non disponibile.",
            "Idioma aplicado mas não guardado: pasta de definições indisponível.",
        ],
    ),
    (
        "Library addition unconfirmed; check your playlists before retrying",
        [
            "Hinzufügen zur Bibliothek unbestätigt; zuerst Playlists prüfen",
            "Adición a la biblioteca sin confirmar; comprueba tus playlists antes de reintentar",
            "Aggiunta alla libreria non confermata; controlla le playlist prima di riprovare",
            "Adição à biblioteca não confirmada; verifica as playlists antes de tentar novamente",
        ],
    ),
    (
        "Library is closing; try again in a moment.",
        [
            "Bibliothek wird geschlossen; gleich erneut versuchen.",
            "La biblioteca se está cerrando; reintenta en un momento.",
            "La libreria si sta chiudendo; riprova tra poco.",
            "A biblioteca está a fechar; tenta de novo dentro de momentos.",
        ],
    ),
    (
        "Link too long",
        [
            "Link zu lang",
            "Enlace demasiado largo",
            "Link troppo lungo",
            "Ligação demasiado longa",
        ],
    ),
    (
        "Loading Spotify playlists exceeded one minute",
        [
            "Laden der Spotify-Playlists dauert länger als eine Minute",
            "La carga de playlists de Spotify superó un minuto",
            "Il caricamento delle playlist Spotify ha superato un minuto",
            "O carregamento de playlists do Spotify excedeu um minuto",
        ],
    ),
    (
        "Loading Spotify tracks...",
        [
            "Spotify-Titel werden geladen...",
            "Cargando canciones de Spotify...",
            "Caricamento brani Spotify...",
            "A carregar faixas do Spotify...",
        ],
    ),
    (
        "Loading audio...",
        [
            "Audio wird geladen...",
            "Cargando audio...",
            "Caricamento audio...",
            "A carregar áudio...",
        ],
    ),
    (
        "Loading playlist...",
        [
            "Playlist wird geladen...",
            "Cargando playlist...",
            "Caricamento playlist...",
            "A carregar playlist...",
        ],
    ),
    (
        "Loading your Spotify playlists...",
        [
            "Deine Spotify-Playlists werden geladen...",
            "Cargando tus playlists de Spotify...",
            "Caricamento delle tue playlist Spotify...",
            "A carregar as tuas playlists do Spotify...",
        ],
    ),
    (
        "Move unconfirmed; refreshing the playlist",
        [
            "Verschieben nicht bestätigt; Playlist wird aktualisiert",
            "Movimiento sin confirmar; actualizando la playlist",
            "Spostamento non confermato; aggiornamento della playlist",
            "Movimento não confirmado; a atualizar a playlist",
        ],
    ),
    (
        "Moving tracks in the Spotify playlist...",
        [
            "Titel in der Spotify-Playlist werden verschoben...",
            "Moviendo canciones en la playlist de Spotify...",
            "Spostamento dei brani nella playlist Spotify...",
            "A mover faixas na playlist Spotify...",
        ],
    ),
    (
        "No accessible music tracks in this link",
        [
            "Keine zugänglichen Musiktitel in diesem Link",
            "No hay canciones accesibles en este enlace",
            "Nessun brano musicale accessibile in questo link",
            "Nenhuma faixa de música acessível nesta ligação",
        ],
    ),
    (
        "No accessible music tracks; the queue was retained",
        [
            "Keine zugänglichen Musiktitel; Warteschlange beibehalten",
            "No hay canciones accesibles; se conservó la cola",
            "Nessun brano musicale accessibile; coda conservata",
            "Nenhuma faixa de música acessível; a fila foi mantida",
        ],
    ),
    (
        "No tracks found. Try another name or artist.",
        [
            "Keine Titel gefunden. Anderen Titel oder Künstler versuchen.",
            "No se encontraron canciones. Prueba otro nombre o artista.",
            "Nessun brano trovato. Prova un altro nome o artista.",
            "Nenhuma música encontrada. Tenta outro nome ou artista.",
        ],
    ),
    (
        "No tracks loaded; try a more specific search",
        [
            "Keine Titel geladen; genauere Suche versuchen",
            "No se cargaron canciones; concreta la búsqueda",
            "Nessun brano caricato; prova una ricerca più precisa",
            "Nenhuma música carregada; tenta uma pesquisa mais precisa",
        ],
    ),
    (
        "Only Spotify track, album and playlist links are supported",
        [
            "Nur Spotify-Links zu Titeln, Alben und Playlists werden unterstützt",
            "Solo se admiten enlaces de Spotify a canciones, álbumes y playlists",
            "Sono supportati solo link Spotify a brani, album e playlist",
            "Só são suportadas ligações do Spotify a faixas, álbuns e playlists",
        ],
    ),
    (
        "Only the owner can move tracks in this playlist",
        [
            "Nur der Eigentümer kann Titel in dieser Playlist verschieben",
            "Solo el propietario puede mover canciones en esta playlist",
            "Solo il proprietario può spostare brani in questa playlist",
            "Só o proprietário pode mover faixas nesta playlist",
        ],
    ),
    (
        "Only the owner can remove tracks from this playlist",
        [
            "Nur der Eigentümer kann Titel aus dieser Playlist entfernen",
            "Solo el propietario puede quitar temas de esta playlist",
            "Solo il proprietario può rimuovere brani da questa playlist",
            "Só o proprietário pode remover faixas desta playlist",
        ],
    ),
    (
        "Opening a browser is not supported on this system.",
        [
            "Dieses System unterstützt das Öffnen eines Browsers nicht.",
            "Este sistema no permite abrir un navegador.",
            "Questo sistema non supporta l'apertura di un browser.",
            "Este sistema não suporta a abertura de um navegador.",
        ],
    ),
    (
        "Opening the project page in your browser...",
        [
            "Projektseite wird im Browser geöffnet...",
            "Abriendo la página del proyecto en el navegador...",
            "Apertura della pagina del progetto nel browser...",
            "A abrir a página do projeto no navegador...",
        ],
    ),
    (
        "Opening the project page was interrupted.",
        [
            "Öffnen der Projektseite wurde unterbrochen.",
            "Se interrumpió la apertura de la página del proyecto.",
            "L'apertura della pagina del progetto è stata interrotta.",
            "A abertura da página do projeto foi interrompida.",
        ],
    ),
    (
        "Paste a Spotify track, album or playlist link",
        [
            "Spotify-Link zu Titel, Album oder Playlist einfügen",
            "Pega un enlace de Spotify a una canción, álbum o playlist",
            "Incolla un link Spotify a un brano, album o playlist",
            "Cola uma ligação do Spotify de faixa, álbum ou playlist",
        ],
    ),
    ("Paused", ["Pausiert", "En pausa", "In pausa", "Em pausa"]),
    (
        "Playback could not be confirmed saved before closing.",
        [
            "Speicherung der Wiedergabe vor dem Schließen konnte nicht bestätigt werden.",
            "No se pudo confirmar el guardado de la reproducción antes de cerrar.",
            "Impossibile confermare il salvataggio della riproduzione prima della chiusura.",
            "Não foi possível confirmar a gravação da reprodução antes de fechar.",
        ],
    ),
    (
        "Playback restored paused. Press Play to resume.",
        [
            "Wiedergabe pausiert wiederhergestellt. Zum Fortfahren Wiedergabe drücken.",
            "Reproducción restaurada en pausa. Pulsa Reproducir para continuar.",
            "Riproduzione ripristinata in pausa. Premi Riproduci per riprendere.",
            "Reprodução restaurada em pausa. Prime Reproduzir para retomar.",
        ],
    ),
    (
        "Playback state unavailable for saving.",
        [
            "Wiedergabestatus zum Speichern nicht verfügbar.",
            "Estado de reproducción no disponible para guardar.",
            "Stato della riproduzione non disponibile per il salvataggio.",
            "Estado da reprodução indisponível para guardar.",
        ],
    ),
    (
        "Playlist change in progress; wait for confirmation",
        [
            "Playlist wird geändert; auf Bestätigung warten",
            "Modificación de playlist en curso; espera la confirmación",
            "Modifica della playlist in corso; attendi la conferma",
            "Alteração da playlist em curso; aguarda a confirmação",
        ],
    ),
    (
        "Preferences directory unavailable",
        [
            "Einstellungsordner nicht verfügbar",
            "Carpeta de preferencias no disponible",
            "Cartella delle preferenze non disponibile",
            "Pasta de preferências indisponível",
        ],
    ),
    (
        "Project page sent to the browser.",
        [
            "Projektseite an den Browser gesendet.",
            "Página del proyecto enviada al navegador.",
            "Pagina del progetto inviata al browser.",
            "Página do projeto enviada para o navegador.",
        ],
    ),
    (
        "Quality applied but not saved: could not write preferences",
        [
            "Qualität angewendet, aber nicht gespeichert: Schreibfehler",
            "Calidad aplicada pero no guardada: no se pudieron escribir las preferencias",
            "Qualità applicata ma non salvata: impossibile scrivere le preferenze",
            "Qualidade aplicada mas não guardada: não foi possível gravar as preferências",
        ],
    ),
    (
        "Queue empty",
        [
            "Warteschlange leer",
            "Cola vacía",
            "Coda vuota",
            "Fila vazia",
        ],
    ),
    (
        "Queue full: 500 tracks maximum",
        [
            "Warteschlange voll: höchstens 500 Titel",
            "Cola llena: máximo 500 canciones",
            "Coda piena: massimo 500 brani",
            "Fila cheia: máximo de 500 faixas",
        ],
    ),
    (
        "Queue full: 500 tracks maximum; the queue was retained",
        [
            "Warteschlange voll: höchstens 500 Titel; Warteschlange beibehalten",
            "Cola llena: máximo 500 canciones; se conservó la cola",
            "Coda piena: massimo 500 brani; coda conservata",
            "Fila cheia: máximo de 500 faixas; a fila foi mantida",
        ],
    ),
    (
        "Reconnecting to your saved Spotify session...",
        [
            "Gespeicherte Spotify-Sitzung wird wieder verbunden...",
            "Reconectando con tu sesión de Spotify guardada...",
            "Riconnessione alla sessione Spotify salvata...",
            "A voltar a ligar à sessão guardada do Spotify...",
        ],
    ),
    (
        "Removal not confirmed; check the playlist before retrying",
        [
            "Entfernen unbestätigt; Playlist vor erneutem Versuch prüfen",
            "Retirada sin confirmar; comprueba la playlist antes de reintentar",
            "Rimozione non confermata; controlla la playlist prima di riprovare",
            "Remoção não confirmada; verifica a playlist antes de tentar novamente",
        ],
    ),
    (
        "Removing tracks from the Spotify playlist...",
        [
            "Titel werden aus der Spotify-Playlist entfernt...",
            "Quitando temas de la playlist de Spotify...",
            "Rimozione dei brani dalla playlist Spotify...",
            "A remover faixas da playlist Spotify...",
        ],
    ),
    (
        "Search incomplete; submit it again to retry",
        [
            "Suche unvollständig; zum Wiederholen erneut suchen",
            "Búsqueda incompleta; envíala de nuevo para reintentar",
            "Ricerca incompleta; inviala di nuovo per riprovare",
            "Pesquisa incompleta; envia-a de novo para tentar novamente",
        ],
    ),
    (
        "Search is limited to 256 characters",
        [
            "Suche auf 256 Zeichen begrenzt",
            "Búsqueda limitada a 256 caracteres",
            "Ricerca limitata a 256 caratteri",
            "Pesquisa limitada a 256 caracteres",
        ],
    ),
    (
        "Searching Spotify...",
        [
            "Spotify wird durchsucht...",
            "Buscando en Spotify...",
            "Ricerca su Spotify...",
            "A pesquisar no Spotify...",
        ],
    ),
    (
        "Selection limited to 500 tracks",
        [
            "Auswahl auf 500 Titel begrenzt",
            "Selección limitada a 500 canciones",
            "Selezione limitata a 500 brani",
            "Seleção limitada a 500 faixas",
        ],
    ),
    (
        "Skin applied but not saved: could not write preferences",
        [
            "Skin angewendet, aber nicht gespeichert: Schreibfehler",
            "Aspecto aplicado pero no guardado: no se pudieron escribir las preferencias",
            "Tema applicato ma non salvato: impossibile scrivere le preferenze",
            "Tema aplicado mas não guardado: não foi possível gravar as preferências",
        ],
    ),
    (
        "Skin applied but not saved: directory unavailable",
        [
            "Skin angewendet, aber nicht gespeichert: Ordner nicht verfügbar",
            "Aspecto aplicado pero no guardado: carpeta no disponible",
            "Tema applicato ma non salvato: cartella non disponibile",
            "Tema aplicado mas não guardado: pasta indisponível",
        ],
    ),
    (
        "Spotify did not provide your playlists' revision",
        [
            "Spotify hat die Versionskennung deiner Playlists nicht geliefert",
            "Spotify no proporcionó la revisión de tus playlists",
            "Spotify non ha fornito la revisione delle tue playlist",
            "O Spotify não forneceu a revisão das tuas playlists",
        ],
    ),
    (
        "Spotify did not respond to the search; try again",
        [
            "Spotify antwortet nicht auf die Suche; erneut versuchen",
            "Spotify no respondió a la búsqueda; inténtalo de nuevo",
            "Spotify non ha risposto alla ricerca; riprova",
            "O Spotify não respondeu à pesquisa; tenta novamente",
        ],
    ),
    (
        "Spotify folder without an identifier",
        [
            "Spotify-Ordner ohne Kennung",
            "Carpeta de Spotify sin identificador",
            "Cartella Spotify senza identificatore",
            "Pasta do Spotify sem identificador",
        ],
    ),
    (
        "Spotify is limiting additions; wait before retrying",
        [
            "Spotify begrenzt das Hinzufügen; vor erneutem Versuch warten",
            "Spotify limita las adiciones; espera antes de reintentar",
            "Spotify limita le aggiunte; attendi prima di riprovare",
            "O Spotify limita as adições; aguarda antes de tentar novamente",
        ],
    ),
    (
        "Spotify is limiting moves; wait before retrying",
        [
            "Spotify begrenzt das Verschieben; vor erneutem Versuch warten",
            "Spotify limita los movimientos; espera antes de reintentar",
            "Spotify limita gli spostamenti; attendi prima di riprovare",
            "O Spotify limita os movimentos; aguarda antes de tentar novamente",
        ],
    ),
    (
        "Spotify is limiting playlist creation; wait before retrying",
        [
            "Spotify begrenzt neue Playlists; vor erneutem Versuch warten",
            "Spotify limita la creación de playlists; espera antes de reintentar",
            "Spotify limita la creazione di playlist; attendi prima di riprovare",
            "O Spotify limita a criação de playlists; aguarda antes de tentar novamente",
        ],
    ),
    (
        "Spotify is limiting removals; wait before retrying",
        [
            "Spotify begrenzt das Entfernen; vor erneutem Versuch warten",
            "Spotify limita las retiradas; espera antes de reintentar",
            "Spotify limita le rimozioni; attendi prima di riprovare",
            "O Spotify limita as remoções; aguarda antes de tentar novamente",
        ],
    ),
    (
        "Spotify is not responding while loading playlists",
        [
            "Spotify reagiert beim Laden der Playlists nicht",
            "Spotify no responde al cargar las playlists",
            "Spotify non risponde durante il caricamento delle playlist",
            "O Spotify não responde ao carregar as playlists",
        ],
    ),
    (
        "Spotify is not responding; try again",
        [
            "Spotify reagiert nicht; erneut versuchen",
            "Spotify no responde; reintenta",
            "Spotify non risponde; riprova",
            "O Spotify não responde; tenta de novo",
        ],
    ),
    (
        "Spotify library limited to 1000 playlists",
        [
            "Spotify-Bibliothek auf 1000 Playlists begrenzt",
            "Biblioteca de Spotify limitada a 1000 playlists",
            "Libreria Spotify limitata a 1000 playlist",
            "Biblioteca do Spotify limitada a 1000 playlists",
        ],
    ),
    (
        "Spotify library limited to 2000 entries, including folders",
        [
            "Spotify-Bibliothek auf 2000 Einträge einschließlich Ordnern begrenzt",
            "Biblioteca de Spotify limitada a 2000 entradas, incluidas carpetas",
            "Libreria Spotify limitata a 2000 elementi, cartelle incluse",
            "Biblioteca do Spotify limitada a 2000 entradas, incluindo pastas",
        ],
    ),
    (
        "Spotify playlist names do not match their entries",
        [
            "Spotify-Playlist-Namen passen nicht zu den Einträgen",
            "Los nombres de las playlists de Spotify no coinciden con sus entradas",
            "I nomi delle playlist Spotify non corrispondono agli elementi",
            "Os nomes das playlists do Spotify não correspondem às entradas",
        ],
    ),
    (
        "Spotify playlist order updated",
        [
            "Reihenfolge der Spotify-Playlist geändert",
            "Orden de la playlist de Spotify actualizado",
            "Ordine della playlist Spotify aggiornato",
            "Ordem da playlist Spotify atualizada",
        ],
    ),
    (
        "Spotify refused the addition; no tracks added",
        [
            "Spotify hat das Hinzufügen abgelehnt; keine Titel hinzugefügt",
            "Spotify rechazó la adición; no se añadieron canciones",
            "Spotify ha rifiutato l'aggiunta; nessun brano aggiunto",
            "O Spotify recusou a adição; nenhuma faixa adicionada",
        ],
    ),
    (
        "Spotify refused the move; no tracks moved",
        [
            "Spotify hat das Verschieben abgelehnt; keine Titel verschoben",
            "Spotify rechazó el movimiento; no se movieron canciones",
            "Spotify ha rifiutato lo spostamento; nessun brano spostato",
            "O Spotify recusou o movimento; nenhuma faixa movida",
        ],
    ),
    (
        "Spotify refused the removal; no tracks removed",
        [
            "Spotify hat das Entfernen abgelehnt; keine Titel entfernt",
            "Spotify rechazó la retirada; no se quitó ningún tema",
            "Spotify ha rifiutato la rimozione; nessun brano rimosso",
            "O Spotify recusou a remoção; nenhuma faixa removida",
        ],
    ),
    (
        "Spotify refused to add it to your library; no tracks added",
        [
            "Spotify hat das Hinzufügen zur Bibliothek abgelehnt; keine Titel hinzugefügt",
            "Spotify rechazó añadirla a tu biblioteca; no se añadieron canciones",
            "Spotify ha rifiutato di aggiungerla alla libreria; nessun brano aggiunto",
            "O Spotify recusou adicioná-la à biblioteca; nenhuma faixa adicionada",
        ],
    ),
    (
        "Spotify refused to create the playlist",
        [
            "Spotify hat die Erstellung der Playlist abgelehnt",
            "Spotify rechazó la creación de la playlist",
            "Spotify ha rifiutato la creazione della playlist",
            "O Spotify recusou a criação da playlist",
        ],
    ),
    (
        "Spotify response too large for the library",
        [
            "Spotify-Antwort zu groß für die Bibliothek",
            "Respuesta de Spotify demasiado grande para la biblioteca",
            "Risposta Spotify troppo grande per la libreria",
            "Resposta do Spotify demasiado grande para a biblioteca",
        ],
    ),
    (
        "Spotify returned an incomplete library; refresh",
        [
            "Spotify hat eine unvollständige Bibliothek geliefert; aktualisieren",
            "Spotify devolvió una biblioteca incompleta; actualiza",
            "Spotify ha restituito una libreria incompleta; aggiorna",
            "O Spotify devolveu uma biblioteca incompleta; atualiza",
        ],
    ),
    (
        "Spotify returned an incomplete page without progress",
        [
            "Spotify hat eine unvollständige Seite ohne Fortschritt geliefert",
            "Spotify devolvió una página incompleta sin avanzar",
            "Spotify ha restituito una pagina incompleta senza avanzamento",
            "O Spotify devolveu uma página incompleta sem progresso",
        ],
    ),
    (
        "Spotify returned an out-of-order playlist page",
        [
            "Spotify hat eine Playlist-Seite in falscher Reihenfolge geliefert",
            "Spotify devolvió una página de playlists desordenada",
            "Spotify ha restituito una pagina di playlist fuori ordine",
            "O Spotify devolveu uma página de playlists fora de ordem",
        ],
    ),
    (
        "Spotify search is unavailable; try again shortly",
        [
            "Spotify-Suche nicht verfügbar; gleich erneut versuchen",
            "Búsqueda de Spotify no disponible; inténtalo en breve",
            "Ricerca Spotify non disponibile; riprova tra poco",
            "Pesquisa Spotify indisponível; tenta dentro de instantes",
        ],
    ),
    (
        "Spotify search response is too large",
        [
            "Spotify-Suchergebnis zu groß",
            "Respuesta de búsqueda de Spotify demasiado grande",
            "Risposta di ricerca Spotify troppo grande",
            "Resposta de pesquisa Spotify demasiado grande",
        ],
    ),
    ("Stopped", ["Gestoppt", "Detenido", "Fermo", "Parado"]),
    (
        "Technical information copied.",
        [
            "Technische Informationen kopiert.",
            "Información técnica copiada.",
            "Informazioni tecniche copiate.",
            "Informações técnicas copiadas.",
        ],
    ),
    (
        "Text clipboard unavailable.",
        [
            "Text-Zwischenablage nicht verfügbar.",
            "Portapapeles de texto no disponible.",
            "Appunti di testo non disponibili.",
            "Área de transferência de texto indisponível.",
        ],
    ),
    (
        "The Spotify library has too many nested folders",
        [
            "Die Spotify-Bibliothek enthält zu viele verschachtelte Ordner",
            "La biblioteca de Spotify tiene demasiadas carpetas anidadas",
            "La libreria Spotify contiene troppe cartelle annidate",
            "A biblioteca do Spotify tem demasiadas pastas aninhadas",
        ],
    ),
    (
        "The Spotify library response could not be read",
        [
            "Die Spotify-Bibliotheksantwort konnte nicht gelesen werden",
            "No se pudo leer la respuesta de la biblioteca de Spotify",
            "Impossibile leggere la risposta della libreria Spotify",
            "Não foi possível ler a resposta da biblioteca do Spotify",
        ],
    ),
    (
        "The audio engine has stopped; restart Spotiamp.",
        [
            "Audio-Engine beendet; Spotiamp neu starten.",
            "El motor de audio se detuvo; reinicia Spotiamp.",
            "Il motore audio si è fermato; riavvia Spotiamp.",
            "O motor de áudio parou; reinicia o Spotiamp.",
        ],
    ),
    (
        "The connection service stopped",
        [
            "Verbindungsdienst beendet",
            "El servicio de conexión se detuvo",
            "Il servizio di connessione si è fermato",
            "O serviço de ligação parou",
        ],
    ),
    (
        "The credential store is not responding",
        [
            "Anmeldespeicher reagiert nicht",
            "El almacén de credenciales no responde",
            "L'archivio credenziali non risponde",
            "O gestor de credenciais não responde",
        ],
    ),
    (
        "The credential store service stopped",
        [
            "Dienst für Anmeldedaten beendet",
            "El servicio de credenciales se detuvo",
            "Il servizio delle credenziali si è fermato",
            "O serviço de credenciais parou",
        ],
    ),
    (
        "The destination is no longer available; refresh your playlists",
        [
            "Ziel nicht mehr verfügbar; Playlists aktualisieren",
            "El destino ya no está disponible; actualiza tus playlists",
            "La destinazione non è più disponibile; aggiorna le playlist",
            "O destino já não está disponível; atualiza as playlists",
        ],
    ),
    (
        "The library window closed after a display error.",
        [
            "Bibliotheksfenster nach einem Anzeigefehler geschlossen.",
            "La ventana de la biblioteca se cerró tras un error de visualización.",
            "La finestra della libreria si è chiusa dopo un errore di visualizzazione.",
            "A janela da biblioteca fechou após um erro de apresentação.",
        ],
    ),
    (
        "The player is busy; try again in a moment.",
        [
            "Player ist beschäftigt; gleich erneut versuchen.",
            "El reproductor está ocupado; reintenta en un momento.",
            "Il lettore è occupato; riprova tra poco.",
            "O leitor está ocupado; tenta de novo dentro de momentos.",
        ],
    ),
    (
        "The playlist changed; refresh before retrying",
        [
            "Playlist wurde geändert; vor erneutem Versuch aktualisieren",
            "La playlist ha cambiado; actualiza antes de reintentar",
            "La playlist è cambiata; aggiorna prima di riprovare",
            "A playlist mudou; atualiza antes de tentar novamente",
        ],
    ),
    (
        "The previous authorization is finishing; try again shortly",
        [
            "Vorherige Autorisierung wird beendet; gleich erneut versuchen",
            "La autorización anterior está terminando; reintenta en un momento",
            "L'autorizzazione precedente sta terminando; riprova tra poco",
            "A autorização anterior está a terminar; tenta de novo dentro de momentos",
        ],
    ),
    (
        "The project page is already opening.",
        [
            "Die Projektseite wird bereits geöffnet.",
            "La página del proyecto ya se está abriendo.",
            "La pagina del progetto si sta già aprendo.",
            "A página do projeto já está a abrir.",
        ],
    ),
    (
        "The queue changed; no tracks moved",
        [
            "Die Warteschlange wurde geändert; keine Titel verschoben",
            "La cola ha cambiado; no se movieron canciones",
            "La coda è cambiata; nessun brano spostato",
            "A fila mudou; nenhuma faixa movida",
        ],
    ),
    (
        "The queue changed; reopen the menu.",
        [
            "Die Warteschlange wurde geändert; Menü erneut öffnen.",
            "La cola ha cambiado; vuelve a abrir el menú.",
            "La coda è cambiata; riapri il menu.",
            "A fila mudou; volta a abrir o menu.",
        ],
    ),
    (
        "The saved session could not be read",
        [
            "Gespeicherte Sitzung konnte nicht gelesen werden",
            "No se pudo leer la sesión guardada",
            "Impossibile leggere la sessione salvata",
            "Não foi possível ler a sessão guardada",
        ],
    ),
    (
        "The session could not be saved",
        [
            "Sitzung konnte nicht gespeichert werden",
            "No se pudo guardar la sesión",
            "Impossibile salvare la sessione",
            "Não foi possível guardar a sessão",
        ],
    ),
    (
        "This Spotify link is not a track, album or playlist",
        [
            "Dieser Spotify-Link führt nicht zu Titel, Album oder Playlist",
            "Este enlace de Spotify no es una canción, álbum o playlist",
            "Questo link Spotify non è un brano, album o playlist",
            "Esta ligação do Spotify não é uma faixa, álbum ou playlist",
        ],
    ),
    (
        "This playlist is read-only; no tracks added",
        [
            "Diese Playlist ist schreibgeschützt; keine Titel hinzugefügt",
            "Esta playlist es de solo lectura; no se añadieron canciones",
            "Questa playlist è di sola lettura; nessun brano aggiunto",
            "Esta playlist é só de leitura; nenhuma faixa adicionada",
        ],
    ),
    (
        "This playlist is unavailable; refresh the library",
        [
            "Diese Playlist ist nicht verfügbar; Bibliothek aktualisieren",
            "Esta playlist no está disponible; actualiza la biblioteca",
            "Questa playlist non è disponibile; aggiorna la libreria",
            "Esta playlist está indisponível; atualiza a biblioteca",
        ],
    ),
    (
        "This result changed; select it again",
        [
            "Dieses Ergebnis wurde geändert; erneut auswählen",
            "Este resultado ha cambiado; selecciónalo de nuevo",
            "Questo risultato è cambiato; selezionalo di nuovo",
            "Este resultado mudou; seleciona-o novamente",
        ],
    ),
    (
        "This track is no longer in the playlist; select it again",
        [
            "Dieser Titel ist nicht mehr in der Playlist; erneut auswählen",
            "Esta canción ya no está en la playlist; selecciónala de nuevo",
            "Questo brano non è più nella playlist; selezionalo di nuovo",
            "Esta faixa já não está na playlist; seleciona-a novamente",
        ],
    ),
    (
        "Track added from search",
        [
            "Titel aus der Suche hinzugefügt",
            "Canción añadida desde la búsqueda",
            "Brano aggiunto dalla ricerca",
            "Música adicionada a partir da pesquisa",
        ],
    ),
    (
        "Track unavailable, access denied or audio error. Try Next.",
        [
            "Titel nicht verfügbar, Zugriff verweigert oder Audiofehler. Weiter versuchen.",
            "Canción no disponible, acceso denegado o error de audio. Prueba Siguiente.",
            "Brano non disponibile, accesso negato o errore audio. Prova Successivo.",
            "Faixa indisponível, acesso recusado ou erro de áudio. Tenta Seguinte.",
        ],
    ),
    (
        "UI MEASUREMENT ONLY - no Spotify engine",
        [
            "NUR UI-MESSUNG - ohne Spotify-Engine",
            "SOLO MEDICIÓN DE INTERFAZ - sin motor Spotify",
            "SOLO MISURA INTERFACCIA - senza motore Spotify",
            "SÓ MEDIÇÃO DA INTERFACE - sem motor Spotify",
        ],
    ),
    (
        "UI measurement build: Spotify engine not compiled.",
        [
            "UI-Messversion: Spotify-Engine nicht kompiliert.",
            "Versión de medición de interfaz: motor Spotify no compilado.",
            "Versione di misura interfaccia: motore Spotify non compilato.",
            "Versão de medição da interface: motor Spotify não compilado.",
        ],
    ),
    (
        "UI measurement build: audio engine not compiled.",
        [
            "UI-Messversion: Audio-Engine nicht kompiliert.",
            "Versión de medición de interfaz: motor de audio no compilado.",
            "Versione di misura interfaccia: motore audio non compilato.",
            "Versão de medição da interface: motor de áudio não compilado.",
        ],
    ),
    (
        "Unable to verify playlist; no tracks removed",
        [
            "Playlist nicht überprüfbar; keine Titel entfernt",
            "No se pudo verificar la playlist; no se quitó ningún tema",
            "Impossibile verificare la playlist; nessun brano rimosso",
            "Não foi possível verificar a playlist; nenhuma faixa removida",
        ],
    ),
    (
        "Update installed.",
        [
            "Update installiert.",
            "Actualización instalada.",
            "Aggiornamento installato.",
            "Atualização instalada.",
        ],
    ),
    (
        "Your playlists changed while loading; refresh",
        [
            "Deine Playlists wurden beim Laden geändert; aktualisieren",
            "Tus playlists cambiaron durante la carga; actualiza",
            "Le tue playlist sono cambiate durante il caricamento; aggiorna",
            "As tuas playlists mudaram durante o carregamento; atualiza",
        ],
    ),
];
