# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Appearance settings carry a Reduce motion choice — follow the system, always or never — so you can
  decide up front whether Sonora animates its interface.
- Appearance settings also carry an Animation speed choice — slow, standard or quick — that stretches
  or tightens every interface animation to taste.

### Changed

- Switches now slide and fade between on and off instead of snapping, at a fixed control speed that
  the animation speed setting deliberately leaves alone.
- A sign-in that fails now explains itself in plain words on a small card, on the login screen and in
  account settings — being outside your account country, an expired session, no connection, a
  cancelled browser approval — instead of showing the raw message from the streaming library. An
  unrecognised failure is trimmed down to one readable sentence too.

## [0.14.0] - 2026-08-14

### Added

- Searching now reaches the whole catalogue for albums and playlists, not just the ones your library
  already knows about. The third column of results holds both, tagged so you can tell them apart,
  and a playlist behaves like one everywhere: open it, play it, pin it or right-click it for the
  usual menu. Albums and playlists you have saved still come first. Spotify and YouTube Music both
  answer these searches from their own catalogue.

### Changed

- The songs, artists and albums columns in search scroll smoothly however many matches come back,
  and each column still scrolls on its own.

### Fixed

- Sonora sat redrawing the window as fast as the screen allowed whenever the lyrics panel was open,
  heating the machine and draining the battery while nothing on screen was moving. It now rests when
  there is nothing to draw.
- When Spotify ships a new build of its web player, album, artist and search pages recover on the
  next attempt instead of failing for hours.

## [0.13.0] - 2026-08-14

### Added

- Sonora remembers what you were listening to. Reopen it and the track you left off on is waiting,
  paused where you stopped, with the rest of the queue and the tracks you already heard still in
  place. It is readied silently in the background, so press play and the music starts from that
  second at once, without the progress bar sliding into place first.
- Double-clicking a word in any text field selects it, and a third click selects everything in the
  field.
- The search field carries a clear button while it holds text, so one click empties it and brings
  the full list back.

### Changed

- The home feed, genre pages and the genre grid in search scroll and resize without stuttering,
  however many shelves the feed carries.
- The single-column search results scroll smoothly however many matches come back, because only the
  rows on screen are drawn.

### Fixed

- Switching music service, or signing out, now clears the queue, the history and the current track,
  so nothing from one service is left sitting in the player when you move to another. Music from
  your imported folder keeps playing.
- YouTube tracks that refused to play now do. When YouTube turns down the quick route, Sonora
  works out the stream signature itself and plays the track anyway, often at a higher bitrate.
- Pinned items now belong to the service they came from, so switching service shows that service's
  pins alongside your imported ones instead of dead rows that failed to open. Switching back brings
  the earlier pins straight back, and existing pins are cleared once on this upgrade.
- Search results no longer all look like the track that is playing: only the song actually playing
  is highlighted and shows a pause button.
- Song titles in search results are readable again instead of being cut after a few letters: the
  three columns only appear once there is real room for them, and a song's duration no longer takes
  space away from its title.
- Every search result answers a right-click now, not just songs. Artists and albums open the same
  context menu they have elsewhere in the app, in the combined list as well as the three columns.
- The genre grid on search scrolls all the way to its end, so the last row of plates no longer sits
  flush against the player bar with its bottom cut off.
- Album, playlist and artist lists in your library end with the same breathing room as every other
  page instead of stopping against the player bar.

## [0.12.1] - 2026-08-13

### Fixed

- The queue scrolls with the same glide as the rest of the app instead of jumping.
- The language picker is wide enough for its search field.

## [0.12.0] - 2026-08-13

### Added

- Search now opens on a grid of genres, and every genre leads to its own page of playlists, albums
  and sub-genres, in cards or in a compact list.
- The home page carries a feed from the service you are signed in to — daily and artist mixes,
  editorial playlists and discovery shelves — with placeholder shelves while it loads, so the page
  is useful before you like a single song.
- German and French translations.
- The language picker has a search field, so a language is one keystroke away instead of a scroll.

### Changed

- Scrolling glides to a stop instead of jumping, at the same speed on any refresh rate. Holding
  Shift over a shelf scrolls it sideways, and a shelf no longer steals a plain wheel scroll.
- Searching keeps what is already in your library above the catalogue results, so a match stops
  sinking the moment the service answers.
- Artist names in the narrow search layout are links again, and a table column stays hidden when
  no row carries a value for it.
- Sonora describes itself as a music streaming client rather than a Spotify client.

### Fixed

- Home, the genre pages and the genre grid only build what is on screen, which removes the stall
  on pages with many covers.
- Opening a genre on YouTube Music no longer crashes on the texture atlas.
- Genre pages that carry nothing playable, and podcast-only entries, disappear from the grid by
  themselves.
- The Nix package installs the released binary instead of wrapping the loader, so audio mixers and
  process lists show Sonora rather than ld-linux.

## [0.11.0] - 2026-08-13

### Added

- Importing a YouTube Music session, or pasting cookies, now asks which Google account to use when
  the session is signed in to more than one, and Sonora stays on the account that was picked.
- The "Paste cookies manually" dialog spells out where the value comes from — which request to open
  in the developer tools, which header to copy, and which cookies the value has to carry.

### Changed

- Importing a browser session is limited to Firefox-based browsers, which the login screen now says
  under the button. Chrome, Edge, Brave and the other Chromium browsers are no longer offered;
  paste the cookies manually to sign in from one of those.
- Connecting a service from the settings now happens there: the cookie dialog, the browser and
  account pickers and a cancel button all appear in Manage accounts, and a failed attempt reports
  the reason on that service's card.
- The queue and the lyrics are opened from a pair of buttons in the player bar instead of the
  floating pills inside the sidebar, and pressing the button of the panel already on screen closes
  the sidebar again.
- An artist page opens with the same card deck the home screen uses for quick picks: 30 popular
  songs across paged columns of five, each card playing the artist's popular list from that song and
  showing the track length. Names below a title appear only where the song is shared with someone
  else. A button in the toolbar switches the section back to the old table, and the choice is
  remembered.
- The card deck keeps one height whatever it holds — a short last page leaves empty rows instead of
  shrinking — and quick picks on the home screen now mixes 30 songs to fill it.
- The artist page reads as an overview: the table view lists five songs and expands to ten, releases
  show two rows and expand to the whole discography, and the artist's biography closes the page in
  the same card the song page uses.
- The "About the artist" card clamps the biography to three lines and opens the full text in a
  dialog; from a song page that dialog also offers a jump to the artist.
- The library lists what changed last first: saved albums carry the date they were saved, playlists
  the date they were last edited, and albums, playlists and followed artists all open sorted by that
  date until a different sort is chosen. Both dates are new sortable columns.
- A narrowing table now gives up one column at a time, in order of how much each column matters,
  instead of dropping three of them at one width and squeezing the rest. The artist of a song is the
  first to go, since the album column carries the same information.

### Fixed

- Connecting a service from the settings no longer throws you onto the login screen, and a failure
  no longer signs you out of the service you were already using.
- Waiting for a browser authorization can be cancelled instead of leaving the app stuck until it is
  restarted, and cancelling now shuts the callback server down. Signing in again used to fail with
  "Address already in use" until Sonora was restarted.
- Cancelling a sign-in no longer flashes the empty library behind the login screen.
- Artwork that does not arrive square — an artist portrait, a cover embedded in a local file — is
  cropped to its middle instead of spilling out of its frame, so a round portrait is round again.
- A pinned table header no longer trembles by a pixel while the page scrolls, and the wheel now
  works over the header and over a column edge instead of stopping there.
- An artist page lists the whole discography — every album, single, compilation and alternate
  edition such as a deluxe or anniversary release — instead of only the most recent releases.

## [0.10.0] - 2026-08-12

### Added

- Anything can be pinned to the sidebar. Drag an album, artist, playlist or song out of a grid, a
  table, a search result, the queue or a page header and drop it into the left sidebar; it stays
  there across restarts, reorders by dragging, and opens a context menu that matches what it is.
- Playlists that arrive without artwork get a cover of their own, stitched together from the first
  four tracks and kept on disk so it is built only once.
- Content cards carry a play control on their artwork — a button in the corner of a tile, a dimmed
  cover on a row — and it stays visible while that item is playing.
- The left sidebar scrolls once its contents outgrow the panel.
- Local music reads more formats, gains its own toolbar, and keeps the list header in place while
  scrolling.
- A playlist that already holds a track marks it, and adding a second copy asks first.

### Changed

- Playlist covers are fetched at the largest size the service offers rather than the smallest.
- The appearance settings put the opacity and adaptive theme switches above the font size.

### Fixed

- The edge used to resize a side panel no longer competes with the row sitting underneath it.
- A playlist that lists the same track more than once keeps every copy.
- The lyrics pane returns to the top when a new track starts, and its scrollbar stays asleep until
  it is needed.
- A menu item that carries only a tick no longer reserves room for an icon beside it.
- Rounded corners nest correctly inside the container that clips them.

## [0.9.0] - 2026-08-11

### Added

- Sonora can play the music already on this device: point it at a folder in the settings and its
  files show up as tracks, albums and artists next to the streaming services.
- The right sidebar shows the lyrics of the playing track next to the queue, switched with a pair of
  floating pills at its foot. Timed lyrics highlight the line being sung, scroll along with it, and
  jump playback to a line when it is clicked.

### Changed

- The right sidebar is opened from the toolbar rather than the player bar, and the player bar button
  in its place opens fullscreen instead.
- A narrow window drops the right sidebar entirely rather than letting it cover the page; lyrics and
  the queue live in fullscreen there.

## [0.8.0] - 2026-08-11

### Added

- Sonora can now sign in to YouTube Music as an alternative to Spotify. The login screen offers
  both services; YouTube Music can be browsed as a guest, connected by importing an existing
  browser session, or connected by pasting cookies, and the whole library — liked songs,
  playlists, albums, artists, search, and radio — works through the same interface.
- The general settings gain a "Manage accounts" section: every service is listed with its own sign
  out, switching to a service already connected takes effect immediately, and a service that is not
  connected yet offers its sign-in options right there — including importing from a named browser.
- The login screen puts each service in its own column with guest mode below them, and importing a
  YouTube Music session asks which browser to read it from — Firefox, Zen, LibreWolf, Floorp,
  Waterfox, Mullvad, Tor, Pale Moon, Basilisk, SeaMonkey, Chrome, Chromium, Brave, Edge, Vivaldi,
  Opera, Yandex, Arc, Thorium and Helium are recognised, including Flatpak and Snap installs.
- YouTube Music albums and playlists play the album audio of a song rather than its music video, so
  a track lasts as long as its listed length.
- Switching, pausing, resuming, and seeking a YouTube Music track fade in and out instead of
  clicking, the previous track stops the moment a new one is picked, and the transport stays
  responsive while the next track loads.

### Changed

- Release cards on an artist page show the release year instead of repeating the artist's name.

### Fixed

- A disabled button reads clearly instead of fading its label into its own background.
- Pasting YouTube Music cookies happens in a dialog that can be dismissed, so a sign-in started by
  mistake no longer leaves the login screen waiting for input.
- Signing out of one service switches to another service still connected, and only falls back to the
  login screen once nothing is connected.
- YouTube Music playlists you own are recognised as yours, so renaming, changing visibility and
  deleting are offered instead of an unsupported "remove from library" that always failed.
- Creating a YouTube Music playlist reports success instead of an error, and the new playlist
  appears straight away rather than after a refresh.
- Removing a track from a YouTube Music playlist works for tracks whose music video was swapped for
  its album audio.
- A YouTube Music playlist you own no longer lists its privacy setting as its owner.
- The player switches to the next YouTube Music track the moment the previous one ends, instead of
  lagging up to half a second behind the audio.
- A YouTube Music session whose saved cookies stopped working falls back to guest browsing or the
  login screen instead of failing with an error.

## [0.7.0] - 2026-08-10

### Added

- Sonora keeps a log file at `$XDG_STATE_HOME/sonora/sonora.log`, so a problem noticed after hours of
  use can still be diagnosed without having started the app from a terminal.
- Playback failures explain themselves: a track that cannot be played is named in a toast, and if
  Spotify refuses the account playback keys entirely, Sonora says so once and stops instead of
  failing through track after track.

### Fixed

- Sonora holds on to far less memory during long listening sessions: cover art it has not shown for
  a while is released instead of being kept until the app closes, and cover art whose page was left
  before the download finished no longer stays in memory for the rest of the session.
- The left sidebar keeps the width you gave it. A window too narrow to fit it now hides it and brings
  it back at that same width, instead of squeezing it down to its narrowest and forgetting the width
  you had; it can still be resized while it sits over the content.
- Card views stay responsive in large libraries: album, song and playlist grids now draw only the
  rows on screen, so resizing the window no longer stutters or reloads covers, and an artist's
  discography opens without a pause.

## [0.6.0] - 2026-08-10

### Added

- Sonora answers the system media controls: media keys, the desktop's now-playing widget and
  lock-screen controls can play, pause, skip, seek and set the volume, and they show the current
  track with its cover art.
- Spotify links open in Sonora: a `spotify:` link to a track, album, playlist or artist opens that
  page, handing it to the window already running instead of starting a second one.

### Changed

- The app presents itself as Sonora rather than sonora, in the window title, the application menu
  and the Windows file properties.

## [0.5.0] - 2026-08-09

### Added

- Albums and playlists can be added to and removed from the library, from their context menu and
  from a heart button beside Play on their page.
- A transparent window background, with an opacity slider under Appearance settings.
- Settings are split into General, Appearance, Playback and About tabs, reachable from the sidebar.
- Copy link actions for tracks, albums, playlists and artists.
- Monthly listeners on the artist page, and play counts beside an artist's popular tracks.
- Toasts confirming queue and playlist additions.
- The volume can be set by scrolling the mouse wheel over the volume control.
- Turning on radio appends what it will play to the queue and lists it under a Similar tracks
  section, picked from the last track you queued. Those tracks play, reorder and can be removed like
  any other queue entry.
- Tooltips on icon-only controls across the player, title bar, toolbar and queue.
- The next track is fetched before the current one ends, so it starts without a gap.
- Gapless playback, on by default and switchable under Playback settings: an album runs from one
  track into the next the way it was sequenced, instead of being cut at the track boundary.

### Changed

- Settings that are on or off are switches instead of text buttons.
- Context menu entries are grouped into sections separated by a rule.
- The sidebar always collapses when the window gets narrow; the setting that governed it is gone.
- Skipping quickly through several tracks only loads the one you stop on.
- The seek and volume sliders have a taller grab area, so they are easier to hit without looking
  any thicker.
- Library cards are virtualized, artist release artwork loads only once it is scrolled to, and the
  artwork image cache is bounded, so large libraries stay responsive.
- Headings, quick pick titles and queue track names are lighter and no longer bold.
- Releases ship static Inter faces instead of the variable font.
- Song, artist and album pages load their details in fewer requests.

### Fixed

- Radio builds its set of similar tracks once, instead of replacing it on every track it plays.
- Durations in search results stay on one line.
- Library cards keep their scroll position when you come back to them.
- The settings menu in the sidebar folds away when you navigate off settings.
- The Play button on a page plays the list as it is shown, so filtering or sorting no longer starts
  a track that is not in view.
- An empty library section says so instead of showing a bare table, and a filter that matches
  nothing says that too.
- Quick picks asks you to like a few songs once the library has loaded, instead of pulsing
  placeholders forever.
- The no-matches note in search lines up with the results above it.
- Clicking a dropdown trigger below the title bar closes its menu instead of leaving it open.
- Releases on the artist page answer to a right-click with the album menu.
- The percentage and timecode bubbles no longer appear below a slider, where a click would not land.
- Rows in quick picks, the queue and search results are inset by the same amount on all four sides,
  instead of drifting a pixel between the top and the bottom.
- Timestamps an hour or longer carry an hours field instead of overflowing the minutes.
- Track menus no longer offer a link to the page that is already open.
- A right-click on a table row no longer also opens the menu behind it.
- Artist release artwork no longer disappears while the page scrolls.
- The library card grid is padded on both sides.
- Play counts of zero are treated as unknown rather than shown as zero.

## [0.4.1] - 2026-08-09

### Added

- An About page crediting the team.

### Changed

- Shuffle, repeat and whether the queue panel is open are remembered between sessions.

### Fixed

- Play next inserts after the current track instead of appending to the queue.
- Resizing one side panel no longer resizes the other.
- A panel responds only to its own drag grip.
- A button label truncates instead of overflowing its button.
- Long song metadata is contained rather than spilling out of its row.

## [0.4.0] - 2026-08-09

### Added

- Playlist management: create, rename, delete, change visibility, and add or remove tracks.
- Albums and playlists can be queued as a whole from their menus.
- Library cards are laid out on a grid that adapts its column count to the space available.
- Toasts above the player report playlist changes.

### Changed

- Item menus are built from one shared definition rather than per-screen copies.
- Track columns are composed from one shared set across every table.
- Playlist edits update the library in place instead of reloading it.
- The library Songs tab stays in list mode, where a card grid adds nothing.
- The text input moved into the design system, and side panels are built on one panel primitive.

### Fixed

- Radio started from a search result is seeded with the track that was picked.
- A newly created playlist is named on creation instead of appearing untitled.

## [0.3.0] - 2026-08-08

### Added

- Columns can be reordered by dragging a header and resized by dragging its edge. Widths, order,
  hidden columns and the active sort are remembered per table between sessions.
- Library sections switch between the table and a card grid through a new View control, remembered
  per section.
- A sort control in the toolbar, so sorting works in card mode as well as in the table.
- Card mode groups rows under first-letter or year headings when the active sort is groupable.
- Album and playlist artwork carries a Play button that starts, pauses and resumes in place.
- Go to album and Go to artist in the track context menu; the latter opens a submenu when a track
  has several artists.
- Queued tracks gained clickable artists, a remove button on hover and the full track menu.
- Liked tracks, with like icons shown on tracks inside albums and playlists.

### Changed

- Track, album and playlist cards are now one primitive rather than four parallel implementations.
- Right-click context menus are a separate element from button dropdowns.
- Every icon was refreshed from Lucide 1.30.0, and the sidebar toggle moved to the panel-left family
  so its divider and arrow match a left-hand panel.
- Album cards show the artist instead of the release type.

### Fixed

- Column headers no longer overlap each other; a column either shows its full heading or is dropped.
- Clicking a menu trigger a second time closes the menu instead of reopening it.
- Selecting an option no longer dismisses the filters, columns or sort dropdown, so several options
  can be picked and the duration slider can be dragged.
- Right-clicking another row moves the context menu in one click.
- Submenus no longer block clicks across the whole window.
- Ghost and outline buttons show a visible hover on the player bar, which is painted in the same
  colour their hover used to be.

## [0.2.0] - 2026-08-07

### Added

- Added an About tab in settings carrying the copyright, the warranty disclaimer and links to the
  license and the source.
- Added `THIRD-PARTY.md`, listing every bundled dependency and the full text of every license.
  Packages and release archives now ship it alongside the Inter and Lucide license files.
- Added a Nix package that installs the prebuilt release binary instead of compiling GPUI.

### Changed

- **Licensing.** Sonora is now released under the GNU General Public License version 3 or later.
  Earlier releases carried no license file at all, which left them undistributable; GPUI depends on
  `zlog` and `ztracing` from the Zed repository, both GPL-3.0-or-later, so every binary ever built
  from this tree was already covered by the GPL. Versions 0.1.0 and 0.1.1 are therefore to be read
  as GPL-3.0-or-later as well.

### Fixed

- Spaced the sidebar tabs apart and drew menus on the popover surface.

## [0.1.1] - 2026-08-07

### Added

- Localized the interface with Fluent, shipping English, Russian, Ukrainian and Polish.
- Added a language setting that follows the system locale by default.
- Added an application icon for macOS, Linux and the disk image.

### Changed

- Releases now ship bare executables for Linux and Windows and a universal disk image for macOS.
- Shuffle moved into the queue, so the queue panel shows the order that will actually play.

### Fixed

- Capped the volume taper at unity gain; the top of the slider no longer clips.
- Aligned the scrubber thumb with the pointer across the whole track.

## [0.1.0] - 2026-08-07

Initial release: a native Spotify client with playback, an interactive queue, the saved library,
search, album, playlist, artist and song pages, context menus and adaptive theming.

[unreleased]: https://github.com/nolight132/sonora/compare/v0.14.0...HEAD
[0.14.0]: https://github.com/nolight132/sonora/compare/v0.13.0...v0.14.0
[0.13.0]: https://github.com/nolight132/sonora/compare/v0.12.1...v0.13.0
[0.12.1]: https://github.com/nolight132/sonora/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/nolight132/sonora/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/nolight132/sonora/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/nolight132/sonora/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/nolight132/sonora/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/nolight132/sonora/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/nolight132/sonora/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/nolight132/sonora/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/nolight132/sonora/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/nolight132/sonora/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/nolight132/sonora/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/nolight132/sonora/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/nolight132/sonora/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/nolight132/sonora/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/nolight132/sonora/releases/tag/v0.1.0
