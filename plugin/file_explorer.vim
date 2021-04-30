let s:BUFFER = '__nerdtree__'

function! s:new_window() abort
  if exists('g:vista_sidebar_open_cmd')
    let open = g:vista_sidebar_open_cmd
  else
    let open = 'vertical topleft '.g:vista_sidebar_width.'new'
  endif

  if get(g:, 'vista_sidebar_keepalt', 0)
    silent execute 'keepalt '.open s:BUFFER
  else
    silent execute open s:BUFFER
  endif

  " Options for a non-file/control buffer.
  setlocal bufhidden=hide
  setlocal buftype=nofile
  setlocal noswapfile

  " Options for controlling buffer/window appearance.
  setlocal foldcolumn=0
  setlocal foldmethod=manual
  setlocal nobuflisted
  setlocal nofoldenable
  setlocal nolist
  setlocal nospell
  setlocal nowrap
  setlocal nomodifiable

  setlocal nonumber
  if v:version >= 703
      setlocal norelativenumber
  endif

  setlocal cursorline

  setlocal filetype=nerdtree

  nnoremap <silent> <buffer> o         :<c-u>call <SID>toggle_action()<CR>
  nnoremap <silent> <buffer> <CR>      :<c-u>call <SID>toggle_action()<CR>
endfunction

function! s:toggle_action() abort
  call clap#client#call('file_explorer/on_toggle', function('s:file_explorer_callback'), {'lnum': line('.'), 'cwd': clap#rooter#working_dir()})
endfunction

function! s:handle_error(error) abort
  echom string(a:error)
endfunction

if has('nvim')

    function! s:setbuflines(bufnr, lines) abort
      call clap#util#nvim_buf_set_lines(a:bufnr, a:lines)
    endfunction

else

    function! s:setbuflines(bufnr, lines) abort
      " silent is required to avoid the annoying --No lines in buffer--.
      silent call deletebufline(a:bufnr, 1, '$')

      call appendbufline(a:bufnr, 0, a:lines)
      " Delete the last possible empty line.
      " Is there a better solution in vim?
      if empty(getbufline(a:bufnr, '$')[0])
        silent call deletebufline(a:bufnr, '$')
      endif
    endfunction
endif

function! s:file_explorer_callback(result, error) abort
  if a:error isnot v:null
    call s:handle_error(a:error)
    return
  endif

  if has_key(a:result, 'file')
    if bufname('') ==# s:BUFFER
      noautocmd wincmd p
    endif
    execute 'edit' a:result.file
    return
  endif

  call setbufvar(g:file_explorer_bufnr, '&modifiable', 1)
  call s:setbuflines(g:file_explorer_bufnr, a:result.lines)
  call setbufvar(g:file_explorer_bufnr, '&modifiable', 0)
endfunction

function! s:init() abort
  call clap#client#call_on_init('file_explorer/on_init', function('s:file_explorer_callback'), {'lnum': line('.'), 'cwd': clap#rooter#working_dir()})
endfunction

" Open or update nerdtree buffer given the rendered rows.
function! FileExplorerOpen() abort
  " (Re)open a window and move to it
  if !exists('g:file_explorer_bufnr')
    call s:new_window()
    let g:file_explorer_bufnr = bufnr('%')
    let g:file_explorer_winid = win_getid()
    let g:file_explorer_pos = [winsaveview(), winnr(), winrestcmd()]
  else
    let winnr = winbufnr(g:file_explorer_bufnr)
    if winnr == -1
      call s:new_window()
    elseif winnr() != winnr
      noautocmd execute winnr.'wincmd w'
    endif
  endif

  " TODO:
  " send request
  call s:init()

  if !g:vista_stay_on_open
    wincmd p
  endif
endfunction

function! FileExplorerClose() abort
  if exists('g:file_explorer_bufnr')
    let winnr = winbufnr(g:file_explorer_bufnr)

    " Jump back to the previous window if we are in the nerdtree sidebar atm.
    if winnr == winnr()
      wincmd p
    endif

    if winnr != -1
      noautocmd execute winnr.'wincmd c'
    endif

    silent execute  g:file_explorer_bufnr.'bwipe!'
    unlet g:file_explorer_bufnr
  endif
endfunction
