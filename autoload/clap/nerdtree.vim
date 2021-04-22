function! s:new_window() abort
  if exists('g:vista_sidebar_open_cmd')
    let open = g:vista_sidebar_open_cmd
  else
    let open = 'vertical topleft '.g:vista_sidebar_width.'new'
  endif

  if get(g:, 'vista_sidebar_keepalt', 0)
    silent execute 'keepalt '.open '__nerdtree__'
  else
    silent execute open '__nerdtree__'
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

  setlocal nonumber
  if v:version >= 703
      setlocal norelativenumber
  endif

  setlocal cursorline

  setlocal filetype=nerdtree

  nnoremap <silent> <buffer> <CR>      :<c-u>call <SID>toggle_action()<CR>
endfunction

function! s:toggle_action() abort
  call clap#client#call('nerdtree/toggle', function('s:nerdtree_callback'), {'lnum': line('.')})
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

function! s:nerdtree_callback(result, error) abort
  if a:error isnot v:null
    call s:handle_error(a:error)
    return
  endif

  call s:setbuflines(g:nerdtree_bufnr, a:result.lines)
endfunction

function! s:notify() abort
  call clap#client#call_on_init('nerdtree', function('s:nerdtree_callback'), {'lnum': line('.'), 'cwd': clap#rooter#working_dir()})
endfunction

" Open or update nerdtree buffer given the rendered rows.
function! clap#nerdtree#open() abort
  " (Re)open a window and move to it
  if !exists('g:nerdtree_bufnr')
    call s:new_window()
    let g:nerdtree_bufnr = bufnr('%')
    let g:nerdtree_winid = win_getid()
    let g:nerdtree_pos = [winsaveview(), winnr(), winrestcmd()]
  else
    let winnr = g:nerdtree_winnr
    if winnr == -1
      call s:new_window()
    elseif winnr() != winnr
      noautocmd execute winnr.'wincmd w'
    endif
  endif

  " TODO:
  " send request
  call s:notify()

  if !g:vista_stay_on_open
    wincmd p
  endif
endfunction

function! clap#nerdtree#close() abort
  if exists('g:nerdtree_bufnr')
    let winnr = g:nerdtree_winnr

    " Jump back to the previous window if we are in the nerdtree sidebar atm.
    if winnr == winnr()
      wincmd p
    endif

    if winnr != -1
      noautocmd execute winnr.'wincmd c'
    endif

    silent execute  g:nerdtree_bufnr.'bwipe!'
    unlet g:nerdtree_bufnr
  endif
endfunction
