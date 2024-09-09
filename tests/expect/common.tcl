proc start_container { exe args } {
    spawn $exe start {*}$args
    expect {
        -re {^([A-Za-z\-]+)\r\n$} { }
        timeout { exit 2 }
        eof { exit 2  }
    }
    wait

    return $expect_out(1,string)
}

proc kill_container { exe args } {
    spawn $exe kill {*}$args
    expect {
        -re {\[[yY]/[nN]\] $} { send "y\r"; send_user "\n" }
        timeout { exit 2 }
        eof { exit 2 }
    }
    wait
}

proc wait_check_result { err_msg { ok_exit_code 0 } } {
    set result [wait]
    set err [lindex $result 3]

    if {[lindex $result 2] == -1} {
        send_user "OS Error: $err\n"
        exit 2
    }

    if {$err != $ok_exit_code} {
        send_user "$err_msg ($err)\n"
        exit 1
    }
}

