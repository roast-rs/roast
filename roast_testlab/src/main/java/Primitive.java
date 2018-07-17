public class Primitive {

	static {
		System.loadLibrary("roast_testlab");
	}

	public static native int addInt(int a, int b);

	public static native boolean compareBool(boolean a, boolean b);

}
