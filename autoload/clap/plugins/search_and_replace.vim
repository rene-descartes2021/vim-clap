" Author: liuchengxu <xuliuchengxlc@gmail.com>
" Description: Search and replace powered by the Rust backend, but not using clap UI.
scriptencoding utf-8

let s:save_cpo = &cpoptions
set cpoptions&vim

function! s:new_window() abort
  " vertical botright 100new
  tabnew
  setlocal
    \ nonumber
    \ norelativenumber
    \ nopaste
    \ nomodeline
    \ noswapfile
    \ nocursorline
    \ nocursorcolumn
    \ colorcolumn=
    \ nobuflisted
    \ buftype=nofile
    \ bufhidden=hide
    \ signcolumn=yes
    \ textwidth=0
    \ nolist
    \ winfixwidth
    \ winfixheight
    \ nospell
    \ nofoldenable
    \ foldcolumn=0
    \ nowrap

  autocmd CursorMoved,CursorMovedI <buffer> call s:on_move()

  let s:winid = win_getid()
endfunction

function! s:handle_data(result, error) abort
  if a:error isnot v:null
    call clap#indicator#set_matches_number(0)
    if has_key(a:error, 'message')
      call g:clap.display.set_lines([a:error.message])
    endif
    return
  endif

  call nvim_buf_set_lines(winbufnr(s:winid), 1, -1, v:true, a:result.lines)

  if has_key(a:result, 'truncated_map')
    let g:__clap_lines_truncated_map = a:result.truncated_map
  endif
endfunction

function! s:on_move() abort
  if line('.') == 1
    let input = getbufline(winbufnr(s:winid), 1, 1)[0]
    let extension = fnamemodify(bufname(g:clap.start.bufnr), ':e')
    call clap#client#call_with_id('dumb_jump/on_typed', function('s:handle_data'), {
          \ 'provider_id': "dumb_jump",
          \ 'query': input,
          \ 'extension': extension,
          \ 'cwd': clap#rooter#working_dir(),
          \ 'display_winwidth': 100,
          \ 'classify': v:true,
          \ }, s:session_id)
  else
    echom 'Try to preview '.line('.')
  endif
endfunction

function! clap#plugins#search_and_replace#run() abort
  if !exists('s:winid')
    call s:new_window()
  endif

  let extension = fnamemodify(bufname(g:clap.start.bufnr), ':e')
  let s:session_id = clap#client#call_on_init(
        \ 'dumb_jump/on_init', function('s:handle_data'), clap#client#init_params({'extension': extension}))
endfunction

let &cpoptions = s:save_cpo
unlet s:save_cpo
