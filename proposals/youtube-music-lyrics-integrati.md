# Proposta — YouTube Music, lyrics integrate e plugin opzionali

**Stato:** proposta prodotto, non approvazione di implementazione  
**Priorità:** dopo PR #1 sicurezza Jam e PR #2 playback/queue  
**Data:** 2026-10-18

## Visione
Tonnomusic deve essere un lettore musicale completo ispirato a SimpMusic, usabile senza installare manualmente plugin per le funzioni base. YouTube Music è un'integrazione desiderata per account, playlist, preferiti, libreria e consigli personalizzati. I lyrics devono essere integrati nell'esperienza.

## Requisiti da studiare

1. **Accesso YouTube Music.** Valutare login e dati utente (playlist, preferiti, libreria, suggerimenti) distinguendo le capacità della YouTube Data API ufficiale dai meccanismi non documentati impiegati da SimpMusic. Non promettere equivalenza YTM o riproduzione audio tramite hifi senza autorizzazione/policy review e prova tecnica.
2. **Plugin non obbligatori.** Ricerca, player, playlist, storico e lyrics disponibili nell'interfaccia di base; nascondere o rendere avanzato il marketplace plugin. Non rimuovere oggi SDK e provider host di Nuclear: mantenere i contratti tecnici interni e le integrazioni/fallback dove utili, senza imporli nell'esperienza utente.
3. **Lyrics built-in.** Verificare le soluzioni SimpMusic: client del servizio `api-lyrics.simpmusic.org/v1`, LRCLIB e BetterLyrics, con testi plain/synced/tradotti, associazione brano-videoID/artista/durata e fallback. L'accessibilità tecnica **non equivale** a un'autorizzazione al riutilizzo del servizio o dei testi: verificare licenze, attribuzione, terms, privacy, rate limit e disponibilità prima di integrare.
4. **Account e privacy.** Per OAuth ufficiale: browser di sistema e token nel secure storage Rust, revoca/disconnessione, nessun cookie/password/token nel renderer o nel plugin SDK.

## Riferimenti tecnici verificati
- [SimpMusicLyricsClient.kt](https://github.com/maxrave-dev/core/blob/82c750da2c2859f5433836ac47b4503f4ec31240/service/lyricsService/src/commonMain/kotlin/org/simpmusic/lyrics/SimpMusicLyricsClient.kt) — query per video ID, traduzioni, LRCLIB/BetterLyrics.
- [SimpMusicLyrics.kt](https://github.com/maxrave-dev/core/blob/82c750da2c2859f5433836ac47b4503f4ec31240/service/lyricsService/src/commonMain/kotlin/org/simpmusic/lyrics/SimpMusicLyrics.kt) — servizio lyrics e provider esterni.
- Nuclear contiene già provider/plugin e un sistema di lyrics; conservarne le capacità sottostanti quando possibile.
- ADR 0001 propone un ecosistema multi-provider, local-first e separa fattibilità, API ufficiali e autorizzazione alla distribuzione.

## Ordine operativo
- **PR #1:** chiudere sicurezza Nuclear Jam, completare CI. Nessun merge automatico.
- **PR #2:** stabilizzare skip, risoluzione stream, queue, playlist e callback asincrone.
- **Poi:** pianificare PR piccole per UX built-in/plugin opzionali, lyrics integrati e integrazione account YouTube con scope verificati. Nessuna modifica applicativa avviata da questa proposta.

## Criteri per approvare la futura implementazione
- Funzionalità fondamentali utilizzabili senza cercare/installare plugin.
- Lyrics visualizzati con eventuali traduzioni e sincronizzazione da fonti autorizzate, senza bloccare la riproduzione quando mancanti.
- Account YouTube con funzioni dichiarate realisticamente, consenso, revoca e gestione errori.
- Playlist e storia locale disponibili anche con account scollegato o servizi fuori uso.
- Review di sicurezza, licenze, ToS/API e dati personali prima della distribuzione.

**Decisioni aperte:** quali fonti lyrics autorizzano il riutilizzo; quali funzioni YouTube Music sono distribuibili; come semplificare l'interfaccia plugin mantenendo compatibilità.
