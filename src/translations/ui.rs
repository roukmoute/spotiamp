//! Static UI labels. Columns: German, Spanish, Italian, Portuguese.
use super::Entry;

pub(super) const ENTRIES: &[Entry] = &[
    ("+ ADD", ["+ HINZ", "+ AÑAD", "+ AGG.", "+ ADIC"]),
    (
        "0.2.10 / REMEMBERED PLAYBACK",
        [
            "0.2.10 / WIEDERGABE GESPEICHERT",
            "0.2.10 / REPRODUCCIÓN GUARDADA",
            "0.2.10 / RIPRODUZIONE SALVATA",
            "0.2.10 / REPRODUÇÃO GUARDADA",
        ],
    ),
    (
        "0.2.11 / SIX LANGUAGES",
        [
            "0.2.11 / SECHS SPRACHEN",
            "0.2.11 / SEIS IDIOMAS",
            "0.2.11 / SEI LINGUE",
            "0.2.11 / SEIS IDIOMAS",
        ],
    ),
    (
        "0.2.12 / SPOTIFY SEARCH",
        [
            "0.2.12 / SPOTIFY-SUCHE",
            "0.2.12 / BÚSQUEDA SPOTIFY",
            "0.2.12 / RICERCA SPOTIFY",
            "0.2.12 / PESQUISA SPOTIFY",
        ],
    ),
    (
        "0.2.6 / MAGNETIC WINDOWS",
        [
            "0.2.6 / MAGNETISCHE FENSTER",
            "0.2.6 / VENTANAS MAGNÉTICAS",
            "0.2.6 / FINESTRE MAGNETICHE",
            "0.2.6 / JANELAS MAGNÉTICAS",
        ],
    ),
    (
        "0.2.7 / REMEMBERED WINDOW LAYOUT",
        [
            "0.2.7 / GESPEICHERTE FENSTERANORDNUNG",
            "0.2.7 / POSICIONES GUARDADAS",
            "0.2.7 / DISPOSIZIONE SALVATA",
            "0.2.7 / DISPOSIÇÃO GUARDADA",
        ],
    ),
    (
        "0.2.8 / NATIVE SPOTIFY PLAYER",
        [
            "0.2.8 / NATIVER SPOTIFY-PLAYER",
            "0.2.8 / REPRODUCTOR NATIVO DE SPOTIFY",
            "0.2.8 / LETTORE SPOTIFY NATIVO",
            "0.2.8 / LEITOR SPOTIFY NATIVO",
        ],
    ),
    (
        "0.2.9 / INTERFACE LANGUAGES",
        [
            "0.2.9 / SPRACHEN DER OBERFLÄCHE",
            "0.2.9 / IDIOMAS DE LA INTERFAZ",
            "0.2.9 / LINGUE DELL'INTERFACCIA",
            "0.2.9 / IDIOMAS DA INTERFACE",
        ],
    ),
    (
        "1. Click CONNECT.",
        [
            "1. Auf VERBINDEN klicken.",
            "1. Haz clic en CONECTAR.",
            "1. Fai clic su CONNETTI.",
            "1. Clica em LIGAR.",
        ],
    ),
    (
        "2. Open your playlists: LIBRARY F3.",
        [
            "2. Playlists öffnen: BIBLIO. F3.",
            "2. Abre tus listas: BIBLIO. F3.",
            "2. Apri le playlist: LIBRER. F3.",
            "2. Abre as playlists: BIBLIO. F3.",
        ],
    ),
    (
        "2. Paste a Spotify link (Ctrl+V).",
        [
            "2. Spotify-Link einfügen (Strg+V).",
            "2. Pega un enlace de Spotify (Ctrl+V).",
            "2. Incolla un link Spotify (Ctrl+V).",
            "2. Cola um link Spotify (Ctrl+V).",
        ],
    ),
    (
        "3. ADD, then double-click to play.",
        [
            "3. HINZ., dann zum Abspielen doppelklicken.",
            "3. AÑADIR y doble clic para reproducir.",
            "3. AGG., poi doppio clic per riprodurre.",
            "3. ADIC. e duplo clique para reproduzir.",
        ],
    ),
    (
        "3. Double-click a playlist to play.",
        [
            "3. Zum Abspielen eine Playlist doppelklicken.",
            "3. Doble clic en una lista para reproducir.",
            "3. Doppio clic su una playlist per riprodurla.",
            "3. Duplo clique numa playlist para reproduzir.",
        ],
    ),
    (
        "A small, lightweight native Spotify player.",
        [
            "Ein kompakter, leichter, nativer Spotify-Player.",
            "Un reproductor Spotify nativo, ligero y compacto.",
            "Un lettore Spotify nativo, leggero e compatto.",
            "Um leitor Spotify nativo, leve e compacto.",
        ],
    ),
    (
        "ABOUT SPOTIAMP",
        [
            "ÜBER SPOTIAMP",
            "ACERCA DE SPOTIAMP",
            "INFORMAZIONI SU SPOTIAMP",
            "SOBRE O SPOTIAMP",
        ],
    ),
    (
        "ADD TO QUEUE",
        ["ANHÄNGEN", "AÑADIR A COLA", "AGGIUNGI", "JUNTAR À FILA"],
    ),
    (
        "About Spotiamp (F2)",
        [
            "Über Spotiamp (F2)",
            "Acerca de Spotiamp (F2)",
            "Informazioni su Spotiamp (F2)",
            "Sobre o Spotiamp (F2)",
        ],
    ),
    (
        "About and documentation updated.",
        [
            "Infofenster und Dokumentation aktualisiert.",
            "Información y documentación actualizadas.",
            "Informazioni e documentazione aggiornate.",
            "Informações e documentação atualizadas.",
        ],
    ),
    (
        "Activate the selected control",
        [
            "Gewähltes Element aktivieren",
            "Activar el control elegido",
            "Attiva il controllo scelto",
            "Ativar o controlo escolhido",
        ],
    ),
    (
        "Add a link to a track, album or playlist.",
        [
            "Link zu Titel, Album oder Playlist hinzufügen.",
            "Añade un enlace a una canción, álbum o lista.",
            "Aggiungi un link a un brano, album o playlist.",
            "Adiciona um link de faixa, álbum ou playlist.",
        ],
    ),
    (
        "Add the link / play",
        [
            "Link hinzufügen / abspielen",
            "Añadir enlace / reproducir",
            "Aggiungi link / riproduci",
            "Adicionar link / reproduzir",
        ],
    ),
    (
        "All names belong to their respective owners.",
        [
            "Alle Namen gehören ihren jeweiligen Inhabern.",
            "Los nombres pertenecen a sus respectivos dueños.",
            "I nomi appartengono ai rispettivi proprietari.",
            "Os nomes pertencem aos respetivos proprietários.",
        ],
    ),
    (
        "An independent project, separate from Spotify/Winamp.",
        [
            "Unabhängiges Projekt, getrennt von Spotify/Winamp.",
            "Proyecto independiente de Spotify y Winamp.",
            "Progetto indipendente da Spotify e Winamp.",
            "Projeto independente do Spotify e do Winamp.",
        ],
    ),
    (
        "Architecture",
        ["Architektur", "Arquitectura", "Architettura", "Arquitetura"],
    ),
    ("Arrows", ["Pfeiltasten", "Flechas", "Frecce", "Setas"]),
    (
        "Arrows / wheel",
        [
            "Pfeile / Mausrad",
            "Flechas / rueda",
            "Frecce / rotellina",
            "Setas / roda",
        ],
    ),
    (
        "Arrows: select / mouse wheel: scroll",
        [
            "Pfeile: auswählen / Mausrad: scrollen",
            "Flechas: elegir / rueda: desplazar",
            "Frecce: seleziona / rotellina: scorri",
            "Setas: selecionar / roda: deslocar",
        ],
    ),
    (
        "Audio decoding",
        [
            "Audiodekodierung",
            "Decodificación de audio",
            "Decodifica audio",
            "Descodificação de áudio",
        ],
    ),
    (
        "Bitmap fonts",
        [
            "Bitmap-Schriften",
            "Fuentes de mapa de bits",
            "Caratteri bitmap",
            "Tipos de letra bitmap",
        ],
    ),
    (
        "Bring the edges together and release to attach them.",
        [
            "Ränder annähern und zum Andocken loslassen.",
            "Acerca los bordes y suelta para unirlas.",
            "Avvicina i bordi e rilascia per agganciarle.",
            "Aproxima as bordas e solta para as unir.",
        ],
    ),
    (
        "Browse the queue",
        [
            "Warteliste durchgehen",
            "Recorrer la cola",
            "Scorri la coda",
            "Percorrer a fila",
        ],
    ),
    ("CANCEL", ["ABBRECHEN", "CANCELAR", "ANNULLA", "CANCELAR"]),
    ("CHECK", ["PRÜFEN", "BUSCAR", "VERIFICA", "VERIFICAR"]),
    ("CLASSIC", ["KLASSIK", "CLASICO", "CLASSICO", "CLASSICO"]),
    ("CLEAR", ["LEEREN", "VACIAR", "SVUOTA", "LIMPAR"]),
    ("CLOSE", ["SCHLIESSEN", "CERRAR", "CHIUDI", "FECHAR"]),
    ("CONNECT", ["VERBINDEN", "CONECTAR", "CONNETTI", "LIGAR"]),
    (
        "CONNECT opens your browser to sign in to Spotify.",
        [
            "VERBINDEN öffnet den Browser zur Spotify-Anmeldung.",
            "CONECTAR abre el navegador para entrar en Spotify.",
            "CONNETTI apre il browser per accedere a Spotify.",
            "LIGAR abre o navegador para entrar no Spotify.",
        ],
    ),
    (
        "COPY INFO",
        ["INFOS KOPIEREN", "COPIAR INFO", "COPIA INFO", "COPIAR INFO"],
    ),
    (
        "Check for a new version of the player.",
        [
            "Nach einer neuen Player-Version suchen.",
            "Busca una nueva versión del reproductor.",
            "Cerca una nuova versione del lettore.",
            "Procura uma nova versão do leitor.",
        ],
    ),
    (
        "Checking for a new version...",
        [
            "Neue Version wird gesucht...",
            "Buscando una nueva versión...",
            "Ricerca di una nuova versione...",
            "A procurar uma nova versão...",
        ],
    ),
    (
        "Choose a playlist in the left column.",
        [
            "Links eine Playlist auswählen.",
            "Elige una lista en la columna izquierda.",
            "Scegli una playlist nella colonna sinistra.",
            "Escolhe uma playlist na coluna da esquerda.",
        ],
    ),
    (
        "Choose a tab or button",
        [
            "Tab oder Schaltfläche wählen",
            "Elegir pestaña o botón",
            "Scegli scheda o pulsante",
            "Escolher separador ou botão",
        ],
    ),
    (
        "Choose in About; remembered across restarts.",
        [
            "Auswahl im Infofenster bleibt beim Neustart erhalten.",
            "Elige en Acerca de; se guarda al reiniciar.",
            "Scegli in Informazioni; la scelta resta al riavvio.",
            "Escolhe em Sobre; a escolha fica guardada.",
        ],
    ),
    (
        "Clipboard",
        [
            "Zwischenablage",
            "Portapapeles",
            "Appunti",
            "Área de transferência",
        ],
    ),
    (
        "Close About",
        [
            "Infofenster schließen",
            "Cerrar Acerca de",
            "Chiudi Informazioni",
            "Fechar Sobre",
        ],
    ),
    (
        "Connect the player",
        [
            "Player verbinden",
            "Conecta el reproductor",
            "Connetti il lettore",
            "Liga o leitor",
        ],
    ),
    (
        "Consistent presentation in the player and help.",
        [
            "Einheitliche Darstellung im Player und in der Hilfe.",
            "Presentación uniforme en reproductor y ayuda.",
            "Aspetto uniforme nel lettore e nella guida.",
            "Apresentação coerente no leitor e na ajuda.",
        ],
    ),
    (
        "Credentials are kept in the system keyring.",
        [
            "Zugangsdaten bleiben im System-Schlüsselspeicher.",
            "Las credenciales se guardan en el llavero del sistema.",
            "Le credenziali restano nel portachiavi di sistema.",
            "As credenciais ficam no cofre do sistema.",
        ],
    ),
    (
        "Credits",
        ["Mitwirkende", "Créditos", "Crediti", "Créditos"],
    ),
    (
        "Ctrl+V / Cmd+V: paste a link",
        [
            "Strg+V / Cmd+V: Link einfügen",
            "Ctrl+V / Cmd+V: pegar un enlace",
            "Ctrl+V / Cmd+V: incolla un link",
            "Ctrl+V / Cmd+V: colar um link",
        ],
    ),
    (
        "Current version:",
        [
            "Aktuelle Version:",
            "Versión actual:",
            "Versione attuale:",
            "Versão atual:",
        ],
    ),
    (
        "DISCONNECT",
        ["TRENNEN", "DESCONECTAR", "DISCONNETTI", "DESLIGAR"],
    ),
    (
        "DISCONNECT removes the saved session.",
        [
            "TRENNEN entfernt die gespeicherte Sitzung.",
            "DESCONECTAR borra la sesión guardada.",
            "DISCONNETTI elimina la sessione salvata.",
            "DESLIGAR remove a sessão guardada.",
        ],
    ),
    (
        "DOWNLOAD",
        ["HERUNTERLADEN", "DESCARGAR", "SCARICA", "TRANSFERIR"],
    ),
    (
        "DOWNLOADING...",
        [
            "DOWNLOAD LÄUFT...",
            "DESCARGANDO...",
            "DOWNLOAD...",
            "A TRANSFERIR...",
        ],
    ),
    ("Delete", ["Entf", "Supr", "Canc", "Delete"]),
    (
        "Delete: remove a track; Tab: link field",
        [
            "Entf: Titel entfernen; Tab: Linkfeld",
            "Supr: quitar canción; Tab: campo de enlace",
            "Canc: rimuovi brano; Tab: campo del link",
            "Delete: remover faixa; Tab: campo do link",
        ],
    ),
    (
        "Dependencies retain their own licenses.",
        [
            "Abhängigkeiten behalten ihre eigenen Lizenzen.",
            "Las dependencias conservan sus propias licencias.",
            "Le dipendenze mantengono le proprie licenze.",
            "As dependências mantêm as suas próprias licenças.",
        ],
    ),
    (
        "Double-click to play. Queue limited to 500 tracks.",
        [
            "Doppelklick: abspielen. Höchstens 500 Titel.",
            "Doble clic: reproducir. Máximo 500 canciones.",
            "Doppio clic: riproduci. Massimo 500 brani.",
            "Duplo clique: reproduzir. Máximo 500 faixas.",
        ],
    ),
    (
        "Double-click: play",
        [
            "Doppelklick: abspielen",
            "Doble clic: reproducir",
            "Doppio clic: riproduci",
            "Duplo clique: reproduzir",
        ],
    ),
    (
        "Double-click: play. Add to queue: append the track.",
        [
            "Doppelklick: abspielen. Zur Liste: Titel hinzufügen.",
            "Doble clic: reproducir. A la cola: añadir canción.",
            "Doppio clic: riproduci. In coda: aggiungi il brano.",
            "Duplo clique: tocar. Na fila: adicionar a faixa.",
        ],
    ),
    (
        "Downloading update (F2)",
        [
            "Update wird geladen (F2)",
            "Descargando actualización (F2)",
            "Download aggiornamento (F2)",
            "A transferir atualização (F2)",
        ],
    ),
    (
        "Drag the library to detach it.",
        [
            "Bibliothek zum Abdocken ziehen.",
            "Arrastra la biblioteca para separarla.",
            "Trascina la libreria per sganciarla.",
            "Arrasta a biblioteca para a separar.",
        ],
    ),
    ("Engine", ["Engine", "Motor", "Motore", "Motor"]),
    ("Enter", ["Eingabe", "Intro", "Invio", "Enter"]),
    (
        "Enter / Space",
        [
            "Eingabe / Leertaste",
            "Intro / Espacio",
            "Invio / Spazio",
            "Enter / Espaço",
        ],
    ),
    (
        "Enter a song or artist.",
        [
            "Titel oder Künstler eingeben.",
            "Escribe una canción o artista.",
            "Inserisci un brano o artista.",
            "Introduz uma música ou artista.",
        ],
    ),
    (
        "Enter: add the link or play the selected track",
        [
            "Eingabe: Link hinzufügen oder Auswahl abspielen",
            "Intro: añadir enlace o reproducir selección",
            "Invio: aggiungi link o riproduci la selezione",
            "Enter: adicionar link ou reproduzir seleção",
        ],
    ),
    (
        "Enter: play    Ctrl+Enter: add to queue",
        [
            "Eingabe: abspielen    Strg+Eingabe: anhängen",
            "Intro: reproducir    Ctrl+Intro: añadir a la cola",
            "Invio: riproduci    Ctrl+Invio: aggiungi alla coda",
            "Enter: reproduzir    Ctrl+Enter: juntar à fila",
        ],
    ),
    (
        "Enter: search    Ctrl+F: search",
        [
            "Eingabe: suchen    Strg+F: Suche",
            "Intro: buscar    Ctrl+F: búsqueda",
            "Invio: cerca    Ctrl+F: ricerca",
            "Enter: pesquisar    Ctrl+F: pesquisa",
        ],
    ),
    (
        "Equalizer is not included.",
        [
            "Equalizer ist nicht enthalten.",
            "No se incluye ecualizador.",
            "Equalizzatore non incluso.",
            "Equalizador não incluído.",
        ],
    ),
    ("Esc / F2", ["Esc / F2", "Esc / F2", "Esc / F2", "Esc / F2"]),
    (
        "Example: https://open.spotify.com/track/...",
        [
            "Beispiel: https://open.spotify.com/track/...",
            "Ejemplo: https://open.spotify.com/track/...",
            "Esempio: https://open.spotify.com/track/...",
            "Exemplo: https://open.spotify.com/track/...",
        ],
    ),
    (
        "F1, Esc or click: close this help.",
        [
            "F1, Esc oder Klick: Hilfe schließen.",
            "F1, Esc o clic: cerrar esta ayuda.",
            "F1, Esc o clic: chiudi la guida.",
            "F1, Esc ou clique: fechar esta ajuda.",
        ],
    ),
    (
        "F2: About, language, credits and shortcuts",
        [
            "F2: Info, Sprache, Mitwirkende und Tastenkürzel",
            "F2: información, idioma, créditos y atajos",
            "F2: informazioni, lingua, crediti e scorciatoie",
            "F2: informações, idioma, créditos e atalhos",
        ],
    ),
    (
        "F3: open your Spotify playlists",
        [
            "F3: Spotify-Playlists öffnen",
            "F3: abrir tus listas de Spotify",
            "F3: apri le tue playlist Spotify",
            "F3: abrir as tuas playlists Spotify",
        ],
    ),
    (
        "Features",
        ["Funktionen", "Funciones", "Funzioni", "Funcionalidades"],
    ),
    (
        "Find a song or artist in the library.",
        [
            "Titel oder Künstler in der Bibliothek suchen.",
            "Busca una canción o artista en la biblioteca.",
            "Cerca un brano o artista nella libreria.",
            "Procure uma música ou artista na biblioteca.",
        ],
    ),
    (
        "Free and open-source software. MIT license.",
        [
            "Freie und quelloffene Software. MIT-Lizenz.",
            "Software libre y de código abierto. Licencia MIT.",
            "Software libero e open source. Licenza MIT.",
            "Software livre e de código aberto. Licença MIT.",
        ],
    ),
    (
        "French and English, with system language detection.",
        [
            "Französisch und Englisch; Systemsprache erkannt.",
            "Francés e inglés; detección del idioma del sistema.",
            "Francese e inglese; lingua del sistema rilevata.",
            "Francês e inglês; deteção do idioma do sistema.",
        ],
    ),
    (
        "French, English, German, Spanish,",
        [
            "Französisch, Englisch, Deutsch, Spanisch,",
            "Francés, inglés, alemán, español,",
            "Francese, inglese, tedesco, spagnolo,",
            "Francês, inglês, alemão, espanhol,",
        ],
    ),
    (
        "Full history: CHANGELOG.md in the source package.",
        [
            "Verlauf: CHANGELOG.md im Quellcodepaket.",
            "Historial completo: CHANGELOG.md en las fuentes.",
            "Cronologia completa: CHANGELOG.md nei sorgenti.",
            "Histórico completo: CHANGELOG.md no código-fonte.",
        ],
    ),
    (
        "GITHUB PROJECT",
        [
            "GITHUB-PROJEKT",
            "PROYECTO GITHUB",
            "PROGETTO GITHUB",
            "PROJETO GITHUB",
        ],
    ),
    (
        "Help / About / Library",
        [
            "Hilfe / Info / Bibliothek",
            "Ayuda / Acerca de / Biblioteca",
            "Guida / Informazioni / Libreria",
            "Ajuda / Sobre / Biblioteca",
        ],
    ),
    (
        "History",
        ["Verlauf", "Historial", "Cronologia", "Histórico"],
    ),
    (
        "IN THE PLAYER",
        ["IM PLAYER", "EN EL REPRODUCTOR", "NEL LETTORE", "NO LEITOR"],
    ),
    (
        "IN THIS WINDOW",
        [
            "IN DIESEM FENSTER",
            "EN ESTA VENTANA",
            "IN QUESTA FINESTRA",
            "NESTA JANELA",
        ],
    ),
    (
        "INSTALL / RESTART",
        [
            "INSTALLIEREN / NEUSTART",
            "INSTALAR / REINICIAR",
            "INSTALLA / RIAVVIA",
            "INSTALAR / REINICIAR",
        ],
    ),
    (
        "INSTALLING...",
        [
            "INSTALLATION...",
            "INSTALANDO...",
            "INSTALLAZIONE...",
            "A INSTALAR...",
        ],
    ),
    (
        "Installing restarts the player and interrupts",
        [
            "Die Installation startet den Player neu und",
            "La instalación reinicia el reproductor e interrumpe",
            "L'installazione riavvia il lettore e interrompe",
            "A instalação reinicia o leitor e interrompe",
        ],
    ),
    (
        "Installing, then restarting...",
        [
            "Installation, danach Neustart...",
            "Instalando; después se reiniciará...",
            "Installazione, poi riavvio...",
            "A instalar, seguido de reinício...",
        ],
    ),
    (
        "Italian and Portuguese. Remembered choice.",
        [
            "Italienisch und Portugiesisch. Auswahl gespeichert.",
            "italiano y portugués. Se recuerda la elección.",
            "italiano e portoghese. La scelta è memorizzata.",
            "italiano e português. A escolha fica guardada.",
        ],
    ),
    ("LANGUAGE", ["SPRACHE", "IDIOMA", "LINGUA", "IDIOMA"]),
    ("LENGTH", ["DAUER", "DURAC.", "DURATA", "DURAÇÃO"]),
    (
        "LIBRARY F3",
        ["BIBLIO. F3", "BIBLIO. F3", "LIBRER. F3", "BIBLIO. F3"],
    ),
    (
        "Library open/closed state restored at startup.",
        [
            "Bibliothek startet wieder geöffnet oder geschlossen.",
            "Se restaura si la biblioteca estaba abierta o cerrada.",
            "La libreria torna aperta o chiusa come prima.",
            "A biblioteca volta aberta ou fechada como antes.",
        ],
    ),
    (
        "Loading track...",
        [
            "Titel wird geladen...",
            "Cargando canción...",
            "Caricamento brano...",
            "A carregar faixa...",
        ],
    ),
    (
        "Loading tracks...",
        [
            "Titel werden geladen...",
            "Cargando canciones...",
            "Caricamento brani...",
            "A carregar faixas...",
        ],
    ),
    (
        "Loading...",
        [
            "Wird geladen...",
            "Cargando...",
            "Caricamento...",
            "A carregar...",
        ],
    ),
    (
        "MY PLAYLISTS",
        [
            "MEINE PLAYLISTS",
            "MIS LISTAS",
            "LE MIE PLAYLIST",
            "AS MINHAS PLAYLISTS",
        ],
    ),
    (
        "Music plays directly in this player.",
        [
            "Musik wird direkt in diesem Player abgespielt.",
            "La música se reproduce directamente aquí.",
            "La musica viene riprodotta direttamente qui.",
            "A música é reproduzida diretamente neste leitor.",
        ],
    ),
    (
        "NATIVE SPOTIFY PLAYER",
        [
            "NATIVER SPOTIFY-PLAYER",
            "REPRODUCTOR SPOTIFY NATIVO",
            "LETTORE SPOTIFY NATIVO",
            "LEITOR SPOTIFY NATIVO",
        ],
    ),
    (
        "Native audio output",
        [
            "Native Audioausgabe",
            "Salida de audio nativa",
            "Uscita audio nativa",
            "Saída de áudio nativa",
        ],
    ),
    ("No", ["Nr.", "N.º", "N.", "N.º"]),
    (
        "No playlists",
        [
            "Keine Playlists",
            "No hay listas",
            "Nessuna playlist",
            "Sem playlists",
        ],
    ),
    (
        "No results.",
        [
            "Keine Ergebnisse.",
            "Sin resultados.",
            "Nessun risultato.",
            "Sem resultados.",
        ],
    ),
    (
        "No tracks loaded.",
        [
            "Keine Titel geladen.",
            "No hay canciones cargadas.",
            "Nessun brano caricato.",
            "Sem faixas carregadas.",
        ],
    ),
    (
        "Not affiliated with Spotify or Winamp.",
        [
            "Nicht mit Spotify oder Winamp verbunden.",
            "Sin afiliación con Spotify ni Winamp.",
            "Non affiliato a Spotify o Winamp.",
            "Sem afiliação ao Spotify ou ao Winamp.",
        ],
    ),
    (
        "Notices and MPL sources are included in the package.",
        [
            "Lizenzhinweise und MPL-Quellcode im Paket enthalten.",
            "Avisos y fuentes MPL incluidos en el paquete.",
            "Avvisi e sorgenti MPL inclusi nel pacchetto.",
            "Avisos e código MPL incluídos no pacote.",
        ],
    ),
    (
        "OFFLINE",
        ["OFFLINE", "SIN CONEXIÓN", "NON CONNESSO", "SEM LIGAÇÃO"],
    ),
    ("PASTE", ["EINF.", "PEGAR", "INCL.", "COLAR"]),
    (
        "PLAY",
        ["ABSPIELEN", "REPRODUCIR", "RIPRODUCI", "REPRODUZIR"],
    ),
    (
        "PLAY QUEUE",
        [
            "WIEDERGABELISTE",
            "COLA DE REPRODUCCION",
            "CODA DI RIPRODUZIONE",
            "FILA DE REPRODUCAO",
        ],
    ),
    (
        "Paste a Spotify link",
        [
            "Spotify-Link einfügen",
            "Pegar enlace de Spotify",
            "Incolla un link Spotify",
            "Colar um link Spotify",
        ],
    ),
    (
        "Paste a Spotify link: track, album...",
        [
            "Spotify-Link einfügen: Titel, Album...",
            "Pega enlace Spotify: canción, álbum...",
            "Incolla link Spotify: brano, album...",
            "Cola um link Spotify: faixa, álbum...",
        ],
    ),
    (
        "Play / pause (outside text input)",
        [
            "Wiedergabe / Pause (ohne Eingabe)",
            "Reproducir / pausa (sin escribir)",
            "Riproduci / pausa (fuori dal testo)",
            "Reproduzir / pausa (fora do texto)",
        ],
    ),
    (
        "Play resumes where you left off.",
        [
            "Wiedergabe setzt an der letzten Position fort.",
            "Reproducir continúa donde lo dejaste.",
            "Riproduci riprende da dove eri rimasto.",
            "Reproduzir retoma onde paraste.",
        ],
    ),
    (
        "Playback continues while downloading.",
        [
            "Die Wiedergabe läuft beim Download weiter.",
            "La reproducción continúa durante la descarga.",
            "La riproduzione continua durante il download.",
            "A reprodução continua durante a transferência.",
        ],
    ),
    (
        "Playlist, track and position restored at startup.",
        [
            "Playlist, Titel und Position beim Start wieder da.",
            "Lista, canción y posición restauradas al iniciar.",
            "Playlist, brano e posizione ripristinati all'avvio.",
            "Playlist, faixa e posição restauradas ao iniciar.",
        ],
    ),
    (
        "Preferred quality",
        [
            "Bevorzugte Qualität",
            "Calidad preferida",
            "Qualità preferita",
            "Qualidade preferida",
        ],
    ),
    (
        "Premium account required. Help: F1",
        [
            "Premium-Konto erforderlich. Hilfe: F1",
            "Se necesita Premium. Ayuda: F1",
            "Account Premium richiesto. Guida: F1",
            "Conta Premium necessária. Ajuda: F1",
        ],
    ),
    (
        "Press F5 to refresh",
        [
            "F5 zum Aktualisieren",
            "Pulsa F5 para actualizar",
            "Premi F5 per aggiornare",
            "Prime F5 para atualizar",
        ],
    ),
    (
        "Preview channel: includes pre-releases.",
        [
            "Vorschaukanal: enthält Vorabversionen.",
            "Canal de pruebas: incluye versiones preliminares.",
            "Canale anteprima: include versioni preliminari.",
            "Canal de testes: inclui versões preliminares.",
        ],
    ),
    ("Project", ["Projekt", "Proyecto", "Progetto", "Projeto"]),
    (
        "Project code: MIT license.",
        [
            "Projektcode: MIT-Lizenz.",
            "Código del proyecto: licencia MIT.",
            "Codice del progetto: licenza MIT.",
            "Código do projeto: licença MIT.",
        ],
    ),
    ("QUEUE", ["WARTELISTE", "COLA", "CODA", "FILA"]),
    (
        "Quality KB/S: 96 / 160 / 320 (default, remembered).",
        [
            "Qualität KB/S: 96 / 160 / 320 (Standard, gespeichert).",
            "Calidad KB/S: 96 / 160 / 320 (por defecto, guardada).",
            "Qualità KB/S: 96 / 160 / 320 (predefinita, salvata).",
            "Qualidade KB/S: 96 / 160 / 320 (padrão, guardada).",
        ],
    ),
    (
        "Quality changes apply on the next audio load.",
        [
            "Neue Qualität gilt beim nächsten Laden eines Titels.",
            "La calidad cambia al cargar el siguiente audio.",
            "La qualità cambia al prossimo caricamento audio.",
            "A qualidade muda no próximo carregamento de áudio.",
        ],
    ),
    (
        "READ ONLY",
        ["NUR LESEN", "SOLO LECTURA", "SOLA LETTURA", "SÓ LEITURA"],
    ),
    (
        "REFRESH",
        ["AKTUALISIEREN", "ACTUALIZAR", "AGGIORNA", "ATUALIZAR"],
    ),
    ("REMOVE", ["ENTFERNEN", "QUITAR", "RIMUOVI", "REMOVER"]),
    (
        "Remove the selected track",
        [
            "Gewählten Titel entfernen",
            "Quitar la canción elegida",
            "Rimuovi il brano scelto",
            "Remover a faixa escolhida",
        ],
    ),
    (
        "Repeat queue: off",
        [
            "Warteliste wiederholen: aus",
            "Repetir cola: no",
            "Ripeti coda: no",
            "Repetir fila: não",
        ],
    ),
    (
        "Repeat queue: on",
        [
            "Warteliste wiederholen: ein",
            "Repetir cola: sí",
            "Ripeti coda: sì",
            "Repetir fila: sim",
        ],
    ),
    ("SEARCH", ["SUCHEN", "BUSCAR", "CERCA", "PESQUISAR"]),
    (
        "SPOTIAMP  /  LIBRARY",
        [
            "SPOTIAMP  /  BIBLIOTHEK",
            "SPOTIAMP  /  BIBLIOTECA",
            "SPOTIAMP  /  LIBRERIA",
            "SPOTIAMP  /  BIBLIOTECA",
        ],
    ),
    (
        "SPOTIAMP / HELP",
        [
            "SPOTIAMP / HILFE",
            "SPOTIAMP / AYUDA",
            "SPOTIAMP / GUIDA",
            "SPOTIAMP / AJUDA",
        ],
    ),
    (
        "SPOTIAMP CONTRIBUTORS",
        [
            "SPOTIAMP-MITWIRKENDE",
            "COLABORADORES DE SPOTIAMP",
            "COLLABORATORI SPOTIAMP",
            "COLABORADORES DO SPOTIAMP",
        ],
    ),
    (
        "SPOTIAMP UPDATES",
        [
            "SPOTIAMP-UPDATES",
            "ACTUALIZACIONES DE SPOTIAMP",
            "AGGIORNAMENTI SPOTIAMP",
            "ATUALIZAÇÕES DO SPOTIAMP",
        ],
    ),
    (
        "SPOTIFY / OFFLINE",
        [
            "SPOTIFY / OFFLINE",
            "SPOTIFY / SIN CONEXIÓN",
            "SPOTIFY / NON CONNESSO",
            "SPOTIFY / SEM LIGAÇÃO",
        ],
    ),
    (
        "SPOTIFY / ONLINE",
        [
            "SPOTIFY / ONLINE",
            "SPOTIFY / CONECTADO",
            "SPOTIFY / CONNESSO",
            "SPOTIFY / LIGADO",
        ],
    ),
    ("Search", ["Suche", "Buscar", "Cerca", "Pesquisa"]),
    (
        "Searching...",
        [
            "Suche läuft...",
            "Buscando...",
            "Ricerca in corso...",
            "A pesquisar...",
        ],
    ),
    (
        "Select a playlist",
        [
            "Playlist auswählen",
            "Elige una lista",
            "Scegli una playlist",
            "Escolhe uma playlist",
        ],
    ),
    ("Shortcuts", ["Kürzel", "Atajos", "Scorciatoie", "Atalhos"]),
    (
        "Show shortcuts",
        [
            "Tastenkürzel anzeigen",
            "Mostrar atajos",
            "Mostra scorciatoie",
            "Mostrar atalhos",
        ],
    ),
    (
        "Shuffle: off",
        [
            "Zufallswiedergabe: aus",
            "Aleatorio: no",
            "Casuale: no",
            "Aleatório: não",
        ],
    ),
    (
        "Shuffle: on",
        [
            "Zufallswiedergabe: ein",
            "Aleatorio: sí",
            "Casuale: sì",
            "Aleatório: sim",
        ],
    ),
    (
        "Sliders: playback position and volume",
        [
            "Regler: Wiedergabeposition und Lautstärke",
            "Barras: posición y volumen",
            "Cursori: posizione e volume",
            "Barras: posição e volume",
        ],
    ),
    ("Space", ["Leertaste", "Espacio", "Spazio", "Espaço"]),
    (
        "Space: play / pause (outside the text field)",
        [
            "Leertaste: Wiedergabe / Pause (ohne Texteingabe)",
            "Espacio: reproducir / pausa (fuera del texto)",
            "Spazio: riproduci / pausa (fuori dal testo)",
            "Espaço: reproduzir / pausa (fora do texto)",
        ],
    ),
    (
        "Spotiamp — Spotify Library",
        [
            "Spotiamp — Spotify-Bibliothek",
            "Spotiamp — Biblioteca de Spotify",
            "Spotiamp — Libreria Spotify",
            "Spotiamp — Biblioteca Spotify",
        ],
    ),
    (
        "Spotify Premium is required to listen.",
        [
            "Spotify Premium ist zum Hören erforderlich.",
            "Se necesita Spotify Premium para escuchar.",
            "Spotify Premium è necessario per ascoltare.",
            "Spotify Premium é necessário para ouvir.",
        ],
    ),
    (
        "Spotify playback",
        [
            "Spotify-Wiedergabe",
            "Reproducción de Spotify",
            "Riproduzione Spotify",
            "Reprodução Spotify",
        ],
    ),
    (
        "Spotify player",
        [
            "Spotify-Player",
            "reproductor de Spotify",
            "lettore Spotify",
            "leitor Spotify",
        ],
    ),
    (
        "Spotify protocol and playback",
        [
            "Spotify-Protokoll und Wiedergabe",
            "Protocolo y reproducción Spotify",
            "Protocollo e riproduzione Spotify",
            "Protocolo e reprodução Spotify",
        ],
    ),
    (
        "Spotify search",
        [
            "Spotify-Suche",
            "Búsqueda Spotify",
            "Ricerca Spotify",
            "Pesquisa Spotify",
        ],
    ),
    (
        "Spotify track (metadata unavailable)",
        [
            "Spotify-Titel (Metadaten fehlen)",
            "Canción Spotify (sin metadatos)",
            "Brano Spotify (metadati assenti)",
            "Faixa Spotify (sem metadados)",
        ],
    ),
    (
        "Stable versions only.",
        [
            "Nur stabile Versionen.",
            "Solo versiones estables.",
            "Solo versioni stabili.",
            "Apenas versões estáveis.",
        ],
    ),
    (
        "Switch skins",
        [
            "Skin wechseln",
            "Cambiar aspecto",
            "Cambia skin",
            "Mudar tema",
        ],
    ),
    (
        "Switch tabs",
        [
            "Tab wechseln",
            "Cambiar pestaña",
            "Cambia scheda",
            "Mudar separador",
        ],
    ),
    ("System", ["System", "Sistema", "Sistema", "Sistema"]),
    (
        "THE JOY OF A SMALL PLAYER.",
        [
            "MUSIKGENUSS MIT KLEINEM PLAYER.",
            "EL PLACER DE UN PEQUEÑO REPRODUCTOR.",
            "IL PIACERE DI UN PICCOLO LETTORE.",
            "O PRAZER DE UM PEQUENO LEITOR.",
        ],
    ),
    ("TIME", ["ZEIT", "TIEMPO", "TEMPO", "TEMPO"]),
    (
        "TITLE / ARTIST",
        [
            "TITEL / INTERPRET",
            "TITULO / ARTISTA",
            "TITOLO / ARTISTA",
            "TITULO / ARTISTA",
        ],
    ),
    ("TRACKS", ["TITEL", "PISTAS", "BRANI", "FAIXAS"]),
    (
        "Tab / Shift+Tab",
        [
            "Tab / Umschalt+Tab",
            "Tab / Mayús+Tab",
            "Tab / Maiusc+Tab",
            "Tab / Shift+Tab",
        ],
    ),
    (
        "The engine may fall back to a lower bitrate.",
        [
            "Der Decoder kann eine geringere Bitrate verwenden.",
            "El motor puede usar una tasa de bits inferior.",
            "Il motore può usare un bitrate inferiore.",
            "O motor pode usar uma taxa de bits inferior.",
        ],
    ),
    (
        "The official Spotify app can stay closed.",
        [
            "Die offizielle Spotify-App kann geschlossen bleiben.",
            "La aplicación oficial de Spotify puede cerrarse.",
            "L'app ufficiale di Spotify può restare chiusa.",
            "A aplicação oficial do Spotify pode ficar fechada.",
        ],
    ),
    (
        "The package is verified before installation.",
        [
            "Das Paket wird vor der Installation überprüft.",
            "El paquete se verifica antes de instalarlo.",
            "Il pacchetto viene verificato prima di installarlo.",
            "O pacote é verificado antes da instalação.",
        ],
    ),
    (
        "The update did not complete.",
        [
            "Das Update wurde nicht abgeschlossen.",
            "La actualización no se completó.",
            "L'aggiornamento non è stato completato.",
            "A atualização não foi concluída.",
        ],
    ),
    (
        "Track count unknown",
        [
            "Titelanzahl unbekannt",
            "Número de canciones desconocido",
            "Numero di brani sconosciuto",
            "Número de faixas desconhecido",
        ],
    ),
    (
        "Tracks load when a playlist is opened.",
        [
            "Titel werden beim Öffnen einer Playlist geladen.",
            "Las canciones se cargan al abrir una lista.",
            "I brani si caricano all'apertura della playlist.",
            "As faixas são carregadas ao abrir uma playlist.",
        ],
    ),
    (
        "Two built-in skins: Classic and Modern.",
        [
            "Zwei integrierte Skins: Klassik und Modern.",
            "Dos aspectos integrados: Clásico y Modern.",
            "Due skin integrate: Classico e Modern.",
            "Dois temas integrados: Clássico e Modern.",
        ],
    ),
    (
        "UI only",
        [
            "Nur Oberfläche",
            "Solo interfaz",
            "Solo interfaccia",
            "Só interface",
        ],
    ),
    (
        "Unavailable",
        [
            "Nicht verfügbar",
            "No disponible",
            "Non disponibile",
            "Indisponível",
        ],
    ),
    (
        "Update available (F2)",
        [
            "Update verfügbar (F2)",
            "Actualización disponible (F2)",
            "Aggiornamento disponibile (F2)",
            "Atualização disponível (F2)",
        ],
    ),
    (
        "Update available - F2",
        [
            "Update verfügbar - F2",
            "Actualización disponible - F2",
            "Aggiornamento disponibile - F2",
            "Atualização disponível - F2",
        ],
    ),
    (
        "Update failed (F2)",
        [
            "Update fehlgeschlagen (F2)",
            "Error de actualización (F2)",
            "Aggiornamento fallito (F2)",
            "Atualização falhou (F2)",
        ],
    ),
    (
        "Update ready - F2",
        [
            "Update bereit - F2",
            "Actualización lista - F2",
            "Aggiornamento pronto - F2",
            "Atualização pronta - F2",
        ],
    ),
    (
        "Update ready to install (F2)",
        [
            "Update installationsbereit (F2)",
            "Actualización lista para instalar (F2)",
            "Aggiornamento pronto da installare (F2)",
            "Atualização pronta a instalar (F2)",
        ],
    ),
    (
        "Updates",
        ["Updates", "Actualizar", "Aggiorna", "Atualizar"],
    ),
    (
        "Updates are unavailable in this build.",
        [
            "Updates sind in dieser Version nicht verfügbar.",
            "Esta versión no permite actualizaciones.",
            "Aggiornamenti non disponibili in questa versione.",
            "Atualizações indisponíveis nesta versão.",
        ],
    ),
    ("VOLUME", ["PEGEL", "VOLUMEN", "VOLUME", "VOLUME"]),
    ("Version", ["Version", "Versión", "Versione", "Versão"]),
    (
        "Window and pixels",
        [
            "Fenster und Pixel",
            "Ventana y píxeles",
            "Finestra e pixel",
            "Janela e píxeis",
        ],
    ),
    (
        "Window positions and attached/free layout remembered.",
        [
            "Fensterpositionen und Andockstatus gespeichert.",
            "Posiciones y ventanas unidas o separadas guardadas.",
            "Posizioni e finestre unite o libere memorizzate.",
            "Posições e janelas unidas ou separadas guardadas.",
        ],
    ),
    (
        "Without a keyring, sign-in cannot be remembered.",
        [
            "Ohne Schlüsselspeicher wird die Anmeldung vergessen.",
            "Sin llavero no se puede recordar la sesión.",
            "Senza portachiavi l'accesso non viene ricordato.",
            "Sem cofre não é possível recordar a sessão.",
        ],
    ),
    (
        "YOUR SPOTIFY PLAYLISTS",
        [
            "DEINE SPOTIFY-PLAYLISTS",
            "TUS LISTAS DE SPOTIFY",
            "LE TUE PLAYLIST SPOTIFY",
            "AS TUAS PLAYLISTS SPOTIFY",
        ],
    ),
    (
        "You have the latest compatible version.",
        [
            "Die neueste kompatible Version ist installiert.",
            "Tienes la última versión compatible.",
            "Hai l'ultima versione compatibile.",
            "Tens a versão compatível mais recente.",
        ],
    ),
    (
        "Your music. Simply.",
        [
            "Deine Musik. Ganz einfach.",
            "Tu música. Así de fácil.",
            "La tua musica. Semplicemente.",
            "A tua música. Sem complicações.",
        ],
    ),
    (
        "none (UI only)",
        [
            "keine (nur Oberfläche)",
            "ninguno (solo interfaz)",
            "nessuno (solo interfaccia)",
            "nenhum (só interface)",
        ],
    ),
    (
        "playback. The queue and position are remembered.",
        [
            "unterbricht die Musik. Warteliste und Position bleiben.",
            "la música. Se guardan la cola y la posición.",
            "la musica. Coda e posizione vengono salvate.",
            "a música. A fila e a posição ficam guardadas.",
        ],
    ),
    (
        "results",
        ["Ergebnisse", "resultados", "risultati", "resultados"],
    ),
    ("tracks", ["Titel", "canciones", "brani", "faixas"]),
];
