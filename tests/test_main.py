from klima.main import main


def test_main(capsys):
    main()
    captured = capsys.readouterr()
    assert "Klima" in captured.out
