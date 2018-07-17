public class Strings {

	static {
		System.loadLibrary("roast_testlab");
	}

	public static native String helloWorld();

	public static native String reverse(String input);

	public static native int countChars(String input);

}
