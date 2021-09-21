" Author: liuchengxu <xuliuchengxlc@gmail.com>
" Description: Jump to definition/reference based on the regexp.

scriptencoding utf-8

let s:save_cpo = &cpoptions
set cpoptions&vim

let s:dumb_jump = {}

function! s:dumb_jump.sink(selected) abort
  let pattern = '^\[\(\a\+\)\]\zs\(.*\):\(\d\+\):\(\d\+\):'
  let matched = matchlist(a:selected, pattern)
  let [fpath, linenr, column] = [matched[2], str2nr(matched[3]), str2nr(matched[4])]
  call clap#sink#open_file(fpath, linenr, column)
endfunction

function! s:into_qf_item(line) abort
  let pattern = '^\[\(\a\+\)\]\zs\(.*\):\(\d\+\):\(\d\+\):\(.*\)'
  let matched = matchlist(a:line, pattern)
  let [fpath, linenr, column, text] = [matched[2], str2nr(matched[3]), str2nr(matched[4]), matched[5]]
  return {'filename': fpath, 'lnum': linenr, 'col': column, 'text': text}
endfunction

function! s:dumb_jump_sink_star(lines) abort
  call clap#util#open_quickfix(map(a:lines, 's:into_qf_item(v:val)'))
endfunction

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
    \ signcolumn=no
    \ textwidth=0
    \ nolist
    \ winfixwidth
    \ winfixheight
    \ nospell
    \ nofoldenable
    \ foldcolumn=0
    \ nowrap

  autocmd CursorMoved,CursorMovedI <buffer> call clap#provider#dumb_jump#on_move()

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

function! clap#provider#dumb_jump#on_move() abort
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

function! clap#provider#dumb_jump#test() abort
  if !exists('s:winid')
    call s:new_window()
  endif

  let extension = fnamemodify(bufname(g:clap.start.bufnr), ':e')
  let s:session_id = clap#client#call_on_init(
        \ 'dumb_jump/on_init', function('s:handle_data'), clap#client#init_params({'extension': extension}))
endfunction

function! s:dumb_jump.on_typed() abort
  let extension = fnamemodify(bufname(g:clap.start.bufnr), ':e')
  call clap#client#call('dumb_jump/on_typed', function('clap#state#handle_response_on_typed'), {
        \ 'provider_id': g:clap.provider.id,
        \ 'query': g:clap.input.get(),
        \ 'extension': extension,
        \ 'cwd': clap#rooter#working_dir(),
        \ })
endfunction

function! s:dumb_jump.init() abort
  let extension = fnamemodify(bufname(g:clap.start.bufnr), ':e')
  call clap#client#call_on_init(
        \ 'dumb_jump/on_init', function('clap#state#handle_response_on_typed'), clap#client#init_params({'extension': extension}))
endfunction

function! s:dumb_jump.on_move_async() abort
  call clap#client#call_with_lnum('dumb_jump/on_move', function('clap#impl#on_move#handler'))
endfunction

let s:dumb_jump['sink*'] = function('s:dumb_jump_sink_star')
let s:dumb_jump.syntax = 'clap_dumb_jump'
let s:dumb_jump.enable_rooter = v:true
let g:clap#provider#dumb_jump# = s:dumb_jump

let &cpoptions = s:save_cpo
unlet s:save_cpo
